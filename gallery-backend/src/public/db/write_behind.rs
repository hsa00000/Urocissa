use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(feature = "performance-test")]
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

use arrayvec::ArrayString;
use rocket::serde::Serialize;
use tokio::sync::Notify;

use crate::public::db::tree::TREE;
use crate::public::db::tree::state::TargetSet;
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::AlbumCombined;
use crate::public::structure::config::{APP_CONFIG, WriteBehindConfig};

const FLUSH_CHUNK_SIZE: usize = 4_096;
const CAPACITY_WAIT: Duration = Duration::from_secs(30);

#[cfg(feature = "performance-test")]
static FAIL_AFTER_COMMITS: AtomicI64 = AtomicI64::new(-1);

#[derive(Debug, Clone)]
pub enum DirtyOperation {
    Tags {
        targets: TargetSet,
        add: BTreeSet<String>,
        remove: BTreeSet<String>,
    },
    Albums {
        targets: TargetSet,
        add: BTreeSet<ArrayString<64>>,
        remove: BTreeSet<ArrayString<64>>,
    },
    Flags {
        targets: TargetSet,
        favorite: Option<bool>,
        archived: Option<bool>,
        trashed: Option<bool>,
    },
    Description {
        target: crate::public::db::tree::state::SlotRef,
        value: Option<String>,
    },
    AlbumCreate(AlbumCombined),
    AlbumReplace(AlbumCombined),
    AlbumDelete(ArrayString<64>),
}

impl DirtyOperation {
    pub fn estimated_bytes(&self) -> usize {
        const HEADER: usize = 96;
        match self {
            Self::Tags {
                targets,
                add,
                remove,
            } => {
                HEADER
                    + targets.estimated_bytes()
                    + add
                        .iter()
                        .chain(remove)
                        .map(String::capacity)
                        .sum::<usize>()
            }
            Self::Albums {
                targets,
                add,
                remove,
            } => {
                HEADER
                    + targets.estimated_bytes()
                    + (add.len() + remove.len()) * std::mem::size_of::<ArrayString<64>>()
            }
            Self::Flags { targets, .. } => HEADER + targets.estimated_bytes(),
            Self::Description { value, .. } => HEADER + value.as_ref().map_or(0, String::capacity),
            Self::AlbumCreate(album) | Self::AlbumReplace(album) => {
                HEADER
                    + std::mem::size_of::<AlbumCombined>()
                    + album.object.thumbhash.as_ref().map_or(0, Vec::capacity)
                    + album
                        .object
                        .description
                        .as_ref()
                        .map_or(0, String::capacity)
                    + album.metadata.title.as_ref().map_or(0, String::capacity)
                    + album
                        .object
                        .tags
                        .iter()
                        .map(String::capacity)
                        .sum::<usize>()
                    + album
                        .metadata
                        .share_list
                        .values()
                        .map(|share| {
                            std::mem::size_of::<crate::public::structure::album::Share>()
                                + share.description.capacity()
                                + share.password.as_ref().map_or(0, String::capacity)
                        })
                        .sum::<usize>()
            }
            Self::AlbumDelete(_) => HEADER,
        }
    }

    fn records(&self) -> usize {
        match self {
            Self::Tags { targets, .. }
            | Self::Albums { targets, .. }
            | Self::Flags { targets, .. } => targets.len(),
            Self::Description { .. }
            | Self::AlbumCreate(_)
            | Self::AlbumReplace(_)
            | Self::AlbumDelete(_) => 1,
        }
    }
}

#[derive(Debug, Clone)]
struct DirtyBatch {
    sequence: u64,
    created: Instant,
    operations: Vec<DirtyOperation>,
    bytes: usize,
}

impl DirtyBatch {
    fn new(sequence: u64) -> Self {
        Self {
            sequence,
            created: Instant::now(),
            operations: Vec::new(),
            bytes: 0,
        }
    }

    fn push(&mut self, operation: DirtyOperation) {
        // Coalesce repeated edits in the active sequence. This is especially
        // important for UI toggles and create -> rename -> share journeys.
        match &operation {
            DirtyOperation::Flags {
                targets,
                favorite,
                archived,
                trashed,
            } => {
                if let Some(index) = self.operations.iter().rposition(|candidate| {
                    matches!(candidate, DirtyOperation::Flags { targets: old, .. } if old == targets)
                }) {
                    let empty = if let DirtyOperation::Flags {
                        favorite: old_favorite,
                        archived: old_archived,
                        trashed: old_trashed,
                        ..
                    } = &mut self.operations[index]
                    {
                        merge_flag_delta(old_favorite, *favorite);
                        merge_flag_delta(old_archived, *archived);
                        merge_flag_delta(old_trashed, *trashed);
                        old_favorite.is_none() && old_archived.is_none() && old_trashed.is_none()
                    } else {
                        false
                    };
                    if empty {
                        self.operations.remove(index);
                    }
                    self.recount_bytes();
                    return;
                }
            }
            DirtyOperation::Description { target, .. } => {
                if let Some(existing) = self.operations.iter_mut().rev().find(|candidate| {
                    matches!(candidate, DirtyOperation::Description { target: old, .. } if old == target)
                }) {
                    *existing = operation;
                    self.recount_bytes();
                    return;
                }
            }
            DirtyOperation::AlbumCreate(album) => {
                let album_id = album.object.id;
                if let Some(existing) = self.operations.iter_mut().find(|candidate| {
                    matches!(candidate, DirtyOperation::AlbumCreate(old) if old.object.id == album_id)
                }) {
                    *existing = operation;
                    self.recount_bytes();
                    return;
                }
                self.operations.retain(
                    |candidate| !matches!(candidate, DirtyOperation::AlbumDelete(old) if *old == album_id),
                );
            }
            DirtyOperation::AlbumReplace(album) => {
                let album_id = album.object.id;
                if let Some(existing) = self.operations.iter_mut().rev().find(|candidate| {
                    matches!(candidate, DirtyOperation::AlbumReplace(old) if old.object.id == album_id)
                }) {
                    *existing = operation;
                    self.recount_bytes();
                    return;
                }
                if let Some(create_index) = self.operations.iter().position(|candidate| {
                    matches!(candidate, DirtyOperation::AlbumCreate(old) if old.object.id == album_id)
                }) {
                    let has_membership_after_create = self.operations[create_index + 1..]
                        .iter()
                        .any(|candidate| {
                            matches!(candidate, DirtyOperation::Albums { add, remove, .. }
                                if add.contains(&album_id) || remove.contains(&album_id))
                        });
                    if !has_membership_after_create {
                        self.operations[create_index] = DirtyOperation::AlbumCreate(album.clone());
                        self.recount_bytes();
                        return;
                    }
                }
                self.operations.retain(
                    |candidate| !matches!(candidate, DirtyOperation::AlbumDelete(old) if *old == album_id),
                );
            }
            DirtyOperation::AlbumDelete(album_id) => {
                let was_pending_create = self.operations.iter().any(|candidate| {
                    matches!(candidate, DirtyOperation::AlbumCreate(old) if old.object.id == *album_id)
                });
                self.operations.retain_mut(|candidate| match candidate {
                    DirtyOperation::AlbumCreate(old) | DirtyOperation::AlbumReplace(old) => {
                        old.object.id != *album_id
                    }
                    DirtyOperation::AlbumDelete(old) => old != album_id,
                    DirtyOperation::Albums { add, remove, .. } if was_pending_create => {
                        add.remove(album_id);
                        remove.remove(album_id);
                        !add.is_empty() || !remove.is_empty()
                    }
                    _ => true,
                });
                if was_pending_create {
                    self.recount_bytes();
                    return;
                }
            }
            DirtyOperation::Tags {
                targets,
                add,
                remove,
            } => {
                if let Some(index) = self.operations.iter().rposition(|candidate| {
                    matches!(candidate, DirtyOperation::Tags { targets: old, .. } if old == targets)
                }) {
                    let empty = if let DirtyOperation::Tags {
                        add: old_add,
                        remove: old_remove,
                        ..
                    } = &mut self.operations[index]
                    {
                        for tag in add {
                            if !old_remove.remove(tag) {
                                old_add.insert(tag.clone());
                            }
                        }
                        for tag in remove {
                            if !old_add.remove(tag) {
                                old_remove.insert(tag.clone());
                            }
                        }
                        old_add.is_empty() && old_remove.is_empty()
                    } else {
                        false
                    };
                    if empty {
                        self.operations.remove(index);
                    }
                    self.recount_bytes();
                    return;
                }
            }
            DirtyOperation::Albums {
                targets,
                add,
                remove,
            } => {
                if let Some(index) = self.operations.iter().rposition(|candidate| {
                    matches!(candidate, DirtyOperation::Albums { targets: old, .. } if old == targets)
                }) {
                    let empty = if let DirtyOperation::Albums {
                        add: old_add,
                        remove: old_remove,
                        ..
                    } = &mut self.operations[index]
                    {
                        for album in add {
                            if !old_remove.remove(album) {
                                old_add.insert(*album);
                            }
                        }
                        for album in remove {
                            if !old_add.remove(album) {
                                old_remove.insert(*album);
                            }
                        }
                        old_add.is_empty() && old_remove.is_empty()
                    } else {
                        false
                    };
                    if empty {
                        self.operations.remove(index);
                    }
                    self.recount_bytes();
                    return;
                }
            }
        }
        self.bytes += operation.estimated_bytes();
        self.operations.push(operation);
    }

    fn recount_bytes(&mut self) {
        self.bytes = self
            .operations
            .iter()
            .map(DirtyOperation::estimated_bytes)
            .sum();
    }

    fn records(&self) -> usize {
        self.operations.iter().map(DirtyOperation::records).sum()
    }

    fn cancel_targets(&mut self, targets: &TargetSet, album_ids: &BTreeSet<ArrayString<64>>) {
        self.operations.retain_mut(|operation| match operation {
            DirtyOperation::Tags {
                targets: pending, ..
            }
            | DirtyOperation::Flags {
                targets: pending, ..
            } => {
                pending.subtract(targets);
                !pending.is_empty()
            }
            DirtyOperation::Albums {
                targets: pending,
                add,
                remove,
            } => {
                pending.subtract(targets);
                add.retain(|album_id| !album_ids.contains(album_id));
                remove.retain(|album_id| !album_ids.contains(album_id));
                !pending.is_empty() && (!add.is_empty() || !remove.is_empty())
            }
            DirtyOperation::Description { target, .. } => !targets.contains(*target),
            DirtyOperation::AlbumCreate(album) | DirtyOperation::AlbumReplace(album) => {
                !album_ids.contains(&album.object.id)
            }
            DirtyOperation::AlbumDelete(album_id) => !album_ids.contains(album_id),
        });
        self.recount_bytes();
    }
}

fn merge_flag_delta(existing: &mut Option<bool>, next: Option<bool>) {
    let Some(next) = next else {
        return;
    };
    if existing.is_some_and(|current| current != next) {
        *existing = None;
    } else {
        *existing = Some(next);
    }
}

#[derive(Debug)]
struct DirtyState {
    active: DirtyBatch,
    flushing: Option<Arc<DirtyBatch>>,
    reserved_bytes: usize,
    next_sequence: u64,
    last_flush_time_ms: Option<u64>,
    last_flush_duration_ms: Option<u64>,
    last_error: Option<String>,
    flush_failure_count: u64,
    flush_retry_count: u64,
    retry_delay: Duration,
    accepting_edits: bool,
    cancelled_flushing_slots: HashSet<crate::public::db::tree::state::SlotRef>,
    cancelled_flushing_albums: BTreeSet<ArrayString<64>>,
}

impl Default for DirtyState {
    fn default() -> Self {
        Self {
            active: DirtyBatch::new(1),
            flushing: None,
            reserved_bytes: 0,
            next_sequence: 2,
            last_flush_time_ms: None,
            last_flush_duration_ms: None,
            last_error: None,
            flush_failure_count: 0,
            flush_retry_count: 0,
            retry_delay: Duration::ZERO,
            accepting_edits: true,
            cancelled_flushing_slots: HashSet::new(),
            cancelled_flushing_albums: BTreeSet::new(),
        }
    }
}

impl DirtyState {
    fn pending_bytes(&self) -> usize {
        self.active.bytes
            + self.flushing.as_ref().map_or(0, |batch| batch.bytes)
            + self.reserved_bytes
    }
}

#[derive(Debug)]
pub struct DirtyDeltaStore {
    state: Mutex<DirtyState>,
    wake_worker: Notify,
    capacity_changed: Notify,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteBehindStatus {
    pub pending_operations: usize,
    pub pending_records: usize,
    pub pending_bytes: usize,
    pub flushing: bool,
    pub oldest_pending_age_ms: Option<u64>,
    pub last_flush_time_ms: Option<u64>,
    pub last_flush_duration_ms: Option<u64>,
    pub last_error: Option<String>,
    pub flush_failure_count: u64,
    pub flush_retry_count: u64,
}

impl DirtyDeltaStore {
    fn new() -> Self {
        Self {
            state: Mutex::new(DirtyState::default()),
            wake_worker: Notify::new(),
            capacity_changed: Notify::new(),
        }
    }

    pub async fn reserve(&self, bytes: usize) -> Result<(), AppError> {
        self.reserve_with_timeout(bytes, CAPACITY_WAIT).await
    }

    async fn reserve_with_timeout(&self, bytes: usize, timeout: Duration) -> Result<(), AppError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let hard_limit = limits().hard_limit_mib * 1024 * 1024;
            {
                let mut state = self.state.lock().unwrap();
                if !state.accepting_edits {
                    return Err(AppError::new(
                        ErrorKind::Unavailable,
                        "write-behind is draining for shutdown",
                    ));
                }
                if state.pending_bytes().saturating_add(bytes) <= hard_limit {
                    state.reserved_bytes += bytes;
                    return Ok(());
                }
            }
            self.wake_worker.notify_one();
            if tokio::time::timeout_at(deadline, self.capacity_changed.notified())
                .await
                .is_err()
            {
                return Err(AppError::new(
                    ErrorKind::Unavailable,
                    "write-behind capacity was not available within 30 seconds",
                )
                .temporary());
            }
        }
    }

    pub fn release_reservation(&self, bytes: usize) {
        let mut state = self.state.lock().unwrap();
        state.reserved_bytes = state.reserved_bytes.saturating_sub(bytes);
        drop(state);
        self.capacity_changed.notify_waiters();
    }

    /// Must be called while the tree mutation write lock is held.
    pub fn enqueue_reserved(&self, operation: DirtyOperation, reserved_bytes: usize) {
        let soft_limit = limits().soft_limit_mib * 1024 * 1024;
        let mut state = self.state.lock().unwrap();
        state.reserved_bytes = state.reserved_bytes.saturating_sub(reserved_bytes);
        state.active.push(operation);
        let should_flush = state.active.bytes >= soft_limit;
        drop(state);
        if should_flush {
            self.wake_worker.notify_one();
        }
    }

    pub fn status(&self) -> WriteBehindStatus {
        let state = self.state.lock().unwrap();
        let active_operations = state.active.operations.len();
        let active_records = state.active.records();
        let (flushing_operations, flushing_records, flushing_created) =
            state.flushing.as_ref().map_or((0, 0, None), |batch| {
                (batch.operations.len(), batch.records(), Some(batch.created))
            });
        let oldest = (!state.active.operations.is_empty())
            .then_some(state.active.created)
            .into_iter()
            .chain(flushing_created)
            .min();
        WriteBehindStatus {
            pending_operations: active_operations + flushing_operations,
            pending_records: active_records + flushing_records,
            pending_bytes: state.pending_bytes(),
            flushing: state.flushing.is_some(),
            oldest_pending_age_ms: oldest
                .map(|created| u64::try_from(created.elapsed().as_millis()).unwrap_or(u64::MAX)),
            last_flush_time_ms: state.last_flush_time_ms,
            last_flush_duration_ms: state.last_flush_duration_ms,
            last_error: state.last_error.clone(),
            flush_failure_count: state.flush_failure_count,
            flush_retry_count: state.flush_retry_count,
        }
    }

    pub fn config_updated(&self) {
        self.wake_worker.notify_one();
        self.capacity_changed.notify_waiters();
    }

    /// Remove metadata patches for objects that are being durably deleted.
    /// A flushing worker may already hold an older Arc, so cancellation is also
    /// recorded and checked after it acquires the persistence lock.
    pub fn cancel_targets(&self, targets: &TargetSet, album_ids: &BTreeSet<ArrayString<64>>) {
        let mut state = self.state.lock().unwrap();
        state.active.cancel_targets(targets, album_ids);
        if state.flushing.is_some() {
            state.cancelled_flushing_slots.extend(targets.iter());
            state
                .cancelled_flushing_albums
                .extend(album_ids.iter().copied());
            if let Some(flushing) = &mut state.flushing {
                Arc::make_mut(flushing).cancel_targets(targets, album_ids);
            }
        }
        drop(state);
        self.capacity_changed.notify_waiters();
    }

    fn is_flushing_slot_cancelled(
        &self,
        slot_ref: crate::public::db::tree::state::SlotRef,
    ) -> bool {
        self.state
            .lock()
            .unwrap()
            .cancelled_flushing_slots
            .contains(&slot_ref)
    }

    fn is_flushing_album_cancelled(&self, album_id: &ArrayString<64>) -> bool {
        self.state
            .lock()
            .unwrap()
            .cancelled_flushing_albums
            .contains(album_id)
    }

    fn rotate(&self) -> Option<Arc<DirtyBatch>> {
        let mut state = self.state.lock().unwrap();
        if let Some(batch) = &state.flushing {
            return Some(Arc::clone(batch));
        }
        if state.active.operations.is_empty() {
            return None;
        }
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        let active = std::mem::replace(&mut state.active, DirtyBatch::new(sequence));
        let batch = Arc::new(active);
        state.flushing = Some(Arc::clone(&batch));
        Some(batch)
    }

    fn complete(&self, sequence: u64, started: Instant, result: &anyhow::Result<()>) {
        let mut state = self.state.lock().unwrap();
        match result {
            Ok(()) => {
                if state
                    .flushing
                    .as_ref()
                    .is_some_and(|batch| batch.sequence == sequence)
                {
                    state.flushing = None;
                }
                state.last_flush_time_ms = Some(unix_time_ms());
                state.last_flush_duration_ms =
                    Some(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
                state.last_error = None;
                state.retry_delay = Duration::ZERO;
                state.cancelled_flushing_slots.clear();
                state.cancelled_flushing_albums.clear();
            }
            Err(error) => {
                state.last_error = Some(format!("{error:#}"));
                state.flush_failure_count = state.flush_failure_count.saturating_add(1);
                state.retry_delay = if state.retry_delay.is_zero() {
                    Duration::from_secs(1)
                } else {
                    (state.retry_delay * 2).min(Duration::from_secs(30))
                };
            }
        }
        drop(state);
        self.capacity_changed.notify_waiters();
    }

    pub fn logical_record(&self, id: &str, durable: Option<AbstractData>) -> Option<AbstractData> {
        let state_guard = TREE.state.read().ok()?;
        let slot_ref = state_guard.find(id);
        drop(state_guard);
        self.logical_record_for_slot(slot_ref, id, durable)
    }

    pub fn logical_record_for_slot(
        &self,
        slot_ref: Option<crate::public::db::tree::state::SlotRef>,
        id: &str,
        durable: Option<AbstractData>,
    ) -> Option<AbstractData> {
        let dirty = self.state.lock().ok()?;
        let mut record = durable;
        if let Some(batch) = &dirty.flushing {
            apply_overlay(&mut record, slot_ref, id, &batch.operations);
        }
        apply_overlay(&mut record, slot_ref, id, &dirty.active.operations);
        record
    }

    pub async fn run_worker(&'static self) {
        loop {
            let retry_delay = self.state.lock().unwrap().retry_delay;
            if retry_delay.is_zero() {
                let config = limits();
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_millis(config.flush_interval_ms)) => {},
                    () = self.wake_worker.notified() => {},
                }
            } else {
                {
                    let mut state = self.state.lock().unwrap();
                    state.flush_retry_count = state.flush_retry_count.saturating_add(1);
                }
                tokio::time::sleep(retry_delay).await;
            }
            let Some(batch) = self.rotate() else {
                continue;
            };
            let sequence = batch.sequence;
            let started = Instant::now();
            let flush_batch = Arc::clone(&batch);
            let result = tokio::task::spawn_blocking(move || persist_batch(&flush_batch))
                .await
                .map_err(anyhow::Error::from)
                .and_then(std::convert::identity);
            self.complete(sequence, started, &result);
            if result.is_ok() && !self.state.lock().unwrap().active.operations.is_empty() {
                self.wake_worker.notify_one();
            }
        }
    }

    pub async fn drain(&self, timeout: Duration) -> bool {
        {
            let mut state = self.state.lock().unwrap();
            state.accepting_edits = false;
        }
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            self.wake_worker.notify_one();
            let status = self.status();
            if status.pending_operations == 0 {
                return true;
            }
            if tokio::time::timeout_at(deadline, self.capacity_changed.notified())
                .await
                .is_err()
            {
                return false;
            }
        }
    }

    pub async fn flush(&self, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            self.wake_worker.notify_one();
            if self.status().pending_operations == 0 {
                return true;
            }
            if tokio::time::timeout_at(deadline, self.capacity_changed.notified())
                .await
                .is_err()
            {
                return false;
            }
        }
    }

    #[cfg(feature = "performance-test")]
    pub fn inject_flush_failure_after_commits(&self, successful_commits: usize) {
        FAIL_AFTER_COMMITS.store(
            i64::try_from(successful_commits).unwrap_or(i64::MAX),
            AtomicOrdering::Release,
        );
        self.wake_worker.notify_one();
    }

    #[cfg(feature = "performance-test")]
    pub fn wake(&self) {
        self.wake_worker.notify_one();
    }
}

pub static WRITE_BEHIND: LazyLock<DirtyDeltaStore> = LazyLock::new(DirtyDeltaStore::new);

fn limits() -> WriteBehindConfig {
    APP_CONFIG
        .get()
        .and_then(|config| {
            config
                .read()
                .ok()
                .map(|config| config.public.write_behind.clone())
        })
        .unwrap_or_default()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

fn slots_to_ids(slots: &[crate::public::db::tree::state::SlotRef]) -> Vec<ArrayString<64>> {
    let state = TREE.state.read().unwrap();
    slots
        .iter()
        .filter_map(|slot_ref| state.get(*slot_ref).map(|record| record.id))
        .collect()
}

#[cfg(feature = "performance-test")]
fn before_flush_commit() -> anyhow::Result<()> {
    loop {
        let remaining = FAIL_AFTER_COMMITS.load(AtomicOrdering::Acquire);
        if remaining < 0 {
            return Ok(());
        }
        if remaining == 0 {
            if FAIL_AFTER_COMMITS
                .compare_exchange(0, -1, AtomicOrdering::AcqRel, AtomicOrdering::Acquire)
                .is_ok()
            {
                anyhow::bail!("injected performance-test write-behind flush failure");
            }
            continue;
        }
        if FAIL_AFTER_COMMITS
            .compare_exchange(
                remaining,
                remaining - 1,
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            )
            .is_ok()
        {
            return Ok(());
        }
    }
}

#[cfg(not(feature = "performance-test"))]
fn before_flush_commit() -> anyhow::Result<()> {
    Ok(())
}

fn persist_batch(batch: &DirtyBatch) -> anyhow::Result<()> {
    let batch_started = Instant::now();
    for operation in &batch.operations {
        let operation_started = Instant::now();
        let operation_name = match operation {
            DirtyOperation::Tags { .. } => "write_behind.flush.tags",
            DirtyOperation::Albums { .. } => "write_behind.flush.albums",
            DirtyOperation::Flags { .. } => "write_behind.flush.flags",
            DirtyOperation::Description { .. } => "write_behind.flush.description",
            DirtyOperation::AlbumCreate(_) => "write_behind.flush.album_create",
            DirtyOperation::AlbumReplace(_) => "write_behind.flush.album_replace",
            DirtyOperation::AlbumDelete(_) => "write_behind.flush.album_delete",
        };
        match operation {
            DirtyOperation::Tags { targets, .. }
            | DirtyOperation::Albums { targets, .. }
            | DirtyOperation::Flags { targets, .. } => {
                let mut slots = targets.iter();
                loop {
                    let mut slot_chunk = slots.by_ref().take(FLUSH_CHUNK_SIZE).collect::<Vec<_>>();
                    if slot_chunk.is_empty() {
                        break;
                    }
                    let _persistence_guard = TREE.persistence_lock.lock().unwrap();
                    slot_chunk
                        .retain(|slot_ref| !WRITE_BEHIND.is_flushing_slot_cancelled(*slot_ref));
                    let ids = slots_to_ids(&slot_chunk);
                    before_flush_commit()?;
                    TREE.store.write(|writer| {
                        for id in &ids {
                            let Some(value) = writer.get(id.as_str())? else {
                                continue;
                            };
                            let mut data = value.value();
                            apply_record_operation(&mut data, operation);
                            writer.insert_at(id.as_str(), &data)?;
                        }
                        Ok::<(), anyhow::Error>(())
                    })?;
                }
            }
            DirtyOperation::Description { target, .. } => {
                let _persistence_guard = TREE.persistence_lock.lock().unwrap();
                if WRITE_BEHIND.is_flushing_slot_cancelled(*target) {
                    continue;
                }
                let ids = slots_to_ids(&[*target]);
                for id in ids {
                    before_flush_commit()?;
                    TREE.store.write(|writer| {
                        if let Some(value) = writer.get(id.as_str())? {
                            let mut data = value.value();
                            apply_record_operation(&mut data, operation);
                            writer.insert_at(id.as_str(), &data)?;
                        }
                        Ok::<(), anyhow::Error>(())
                    })?;
                }
            }
            DirtyOperation::AlbumCreate(album) | DirtyOperation::AlbumReplace(album) => {
                let _persistence_guard = TREE.persistence_lock.lock().unwrap();
                if WRITE_BEHIND.is_flushing_album_cancelled(&album.object.id) {
                    continue;
                }
                before_flush_commit()?;
                TREE.store.write(|writer| {
                    writer.insert_at(
                        album.object.id.as_str(),
                        &AbstractData::Album(album.clone()),
                    )
                })?;
            }
            DirtyOperation::AlbumDelete(album_id) => {
                let _persistence_guard = TREE.persistence_lock.lock().unwrap();
                if WRITE_BEHIND.is_flushing_album_cancelled(album_id) {
                    continue;
                }
                before_flush_commit()?;
                TREE.store
                    .write(|writer| writer.remove(album_id.as_str()))?;
            }
        }
        crate::perf_timing!(operation_name, operation_started, "Flush dirty operation");
    }
    crate::perf_timing!(
        "write_behind.flush.batch",
        batch_started,
        "Flush dirty batch sequence {}",
        batch.sequence
    );
    Ok(())
}

fn apply_record_operation(data: &mut AbstractData, operation: &DirtyOperation) {
    match operation {
        DirtyOperation::Tags { add, remove, .. } => {
            let tags = data.tag_mut();
            tags.extend(add.iter().cloned());
            for tag in remove {
                tags.remove(tag);
            }
        }
        DirtyOperation::Albums { add, remove, .. } => {
            if let Some(albums) = data.albums_mut() {
                albums.extend(add.iter().copied());
                for album in remove {
                    albums.remove(album);
                }
            }
        }
        DirtyOperation::Flags {
            favorite,
            archived,
            trashed,
            ..
        } => {
            if let Some(value) = favorite {
                data.set_favorite(*value);
            }
            if let Some(value) = archived {
                data.set_archived(*value);
            }
            if let Some(value) = trashed {
                data.set_trashed(*value);
            }
        }
        DirtyOperation::Description { value, .. } => match data {
            AbstractData::Image(item) => item.object.description.clone_from(value),
            AbstractData::Video(item) => item.object.description.clone_from(value),
            AbstractData::Album(item) => item.object.description.clone_from(value),
        },
        DirtyOperation::AlbumCreate(_)
        | DirtyOperation::AlbumReplace(_)
        | DirtyOperation::AlbumDelete(_) => {}
    }
}

fn apply_overlay(
    record: &mut Option<AbstractData>,
    slot_ref: Option<crate::public::db::tree::state::SlotRef>,
    id: &str,
    operations: &[DirtyOperation],
) {
    for operation in operations {
        match operation {
            DirtyOperation::AlbumCreate(album) | DirtyOperation::AlbumReplace(album)
                if album.object.id.as_str() == id =>
            {
                *record = Some(AbstractData::Album(album.clone()));
            }
            DirtyOperation::AlbumDelete(album_id) if album_id.as_str() == id => {
                *record = None;
            }
            DirtyOperation::Tags { targets, .. }
            | DirtyOperation::Albums { targets, .. }
            | DirtyOperation::Flags { targets, .. }
                if slot_ref.is_some_and(|slot_ref| targets.contains(slot_ref)) =>
            {
                if let Some(data) = record {
                    apply_record_operation(data, operation);
                }
            }
            DirtyOperation::Description { target, .. } if slot_ref == Some(*target) => {
                if let Some(data) = record {
                    apply_record_operation(data, operation);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::db::tree::state::SlotRef;
    use crate::public::structure::album::Album;

    fn targets(values: impl IntoIterator<Item = u32>) -> TargetSet {
        TargetSet::from_slot_refs(values.into_iter().map(|value| SlotRef::new(value, 1)), 64)
    }

    #[test]
    fn coalesces_inverse_flag_deltas_and_latest_descriptions() {
        let targets = targets([1, 2, 3]);
        let mut batch = DirtyBatch::new(1);
        batch.push(DirtyOperation::Flags {
            targets: targets.clone(),
            favorite: Some(true),
            archived: None,
            trashed: None,
        });
        batch.push(DirtyOperation::Flags {
            targets,
            favorite: Some(false),
            archived: Some(true),
            trashed: None,
        });
        batch.push(DirtyOperation::Description {
            target: SlotRef::new(1, 1),
            value: Some("old".to_owned()),
        });
        batch.push(DirtyOperation::Description {
            target: SlotRef::new(1, 1),
            value: Some("new".to_owned()),
        });
        assert_eq!(batch.operations.len(), 2);
        assert!(matches!(
            &batch.operations[0],
            DirtyOperation::Flags {
                favorite: None,
                archived: Some(true),
                ..
            }
        ));
        assert!(matches!(
            &batch.operations[1],
            DirtyOperation::Description { value: Some(value), .. } if value == "new"
        ));
        assert_eq!(
            batch.bytes,
            batch
                .operations
                .iter()
                .map(DirtyOperation::estimated_bytes)
                .sum::<usize>()
        );
    }

    #[test]
    fn coalesces_inverse_tag_edits_to_the_final_value() {
        let targets = targets([1, 2]);
        let mut batch = DirtyBatch::new(1);
        batch.push(DirtyOperation::Tags {
            targets: targets.clone(),
            add: BTreeSet::from(["marker".to_owned()]),
            remove: BTreeSet::new(),
        });
        batch.push(DirtyOperation::Tags {
            targets,
            add: BTreeSet::new(),
            remove: BTreeSet::from(["marker".to_owned()]),
        });
        assert!(batch.operations.is_empty());
        assert_eq!(batch.bytes, 0);
    }

    #[test]
    fn unflushed_album_create_delete_cancels_membership_and_records() {
        let album_id = ArrayString::<64>::from("pending-album").unwrap();
        let album = match Album::new(album_id, Some("Pending".to_owned())).into_abstract_data() {
            AbstractData::Album(album) => album,
            _ => unreachable!(),
        };
        let mut batch = DirtyBatch::new(1);
        batch.push(DirtyOperation::AlbumCreate(album.clone()));
        batch.push(DirtyOperation::Albums {
            targets: targets([1, 2]),
            add: BTreeSet::from([album_id]),
            remove: BTreeSet::new(),
        });
        batch.push(DirtyOperation::AlbumReplace(album));
        batch.push(DirtyOperation::AlbumDelete(album_id));
        assert!(batch.operations.is_empty());
        assert_eq!(batch.bytes, 0);
    }

    #[tokio::test]
    async fn hard_limit_wait_returns_service_unavailable() {
        let store = DirtyDeltaStore::new();
        store.state.lock().unwrap().reserved_bytes = usize::MAX / 2;
        let error = store
            .reserve_with_timeout(1, Duration::from_millis(5))
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Unavailable);
        assert!(matches!(
            error.status,
            crate::public::error::ErrorStatus::Temporary
        ));
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn injected_chunk_failure_is_one_shot() {
        WRITE_BEHIND.inject_flush_failure_after_commits(1);
        assert!(before_flush_commit().is_ok());
        assert!(before_flush_commit().is_err());
        assert!(before_flush_commit().is_ok());
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn metadata_patch_preserves_full_exif_and_static_metadata() {
        let mut data = AbstractData::generate_performance_data(42, 7);
        data.exif_vec_mut()
            .unwrap()
            .insert("MakerNote".to_owned(), "opaque-camera-data".to_owned());
        let exif = data.exif_vec().unwrap().clone();
        let aliases = data.alias().to_vec();
        let width = data.width();
        let height = data.height();
        let extension = data.ext().to_owned();
        let slot = SlotRef::new(1, 1);
        let target_set = TargetSet::from_slot_refs([slot], 64);

        apply_record_operation(
            &mut data,
            &DirtyOperation::Tags {
                targets: target_set.clone(),
                add: BTreeSet::from(["new-tag".to_owned()]),
                remove: BTreeSet::new(),
            },
        );
        apply_record_operation(
            &mut data,
            &DirtyOperation::Flags {
                targets: target_set,
                favorite: Some(true),
                archived: Some(true),
                trashed: Some(false),
            },
        );
        apply_record_operation(
            &mut data,
            &DirtyOperation::Description {
                target: slot,
                value: Some("patched".to_owned()),
            },
        );

        assert_eq!(data.exif_vec().unwrap(), &exif);
        assert_eq!(data.alias(), aliases);
        assert_eq!(data.width(), width);
        assert_eq!(data.height(), height);
        assert_eq!(data.ext(), extension);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn active_overlay_wins_over_flushing_sequence() {
        let mut record = Some(AbstractData::generate_performance_data(7, 11));
        let id = record.as_ref().unwrap().hash().to_string();
        let slot = SlotRef::new(3, 1);
        let targets = TargetSet::from_slot_refs([slot], 64);
        let flushing = [DirtyOperation::Description {
            target: slot,
            value: Some("flushing".to_owned()),
        }];
        let active = [DirtyOperation::Description {
            target: slot,
            value: Some("active".to_owned()),
        }];

        apply_overlay(&mut record, Some(slot), &id, &flushing);
        apply_overlay(&mut record, Some(slot), &id, &active);
        let description = match record.unwrap() {
            AbstractData::Image(item) => item.object.description,
            AbstractData::Video(item) => item.object.description,
            AbstractData::Album(item) => item.object.description,
        };
        assert_eq!(description.as_deref(), Some("active"));
        assert!(targets.contains(slot));
    }
}
