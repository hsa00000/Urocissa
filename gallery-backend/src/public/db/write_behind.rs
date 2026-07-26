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
use crate::storage::store::RecordWriter;
use crate::storage::v6::V6AbstractData;

pub const FLUSH_CHUNK_SIZE: usize = 8_192;
const CAPACITY_WAIT: Duration = Duration::from_secs(30);
const FLUSH_RATE_EWMA_ALPHA: f64 = 0.25;

#[cfg(feature = "performance-test")]
static FAIL_AFTER_COMMITS: AtomicI64 = AtomicI64::new(-1);

#[derive(Debug, Clone)]
pub enum DirtyOperation {
    Touch {
        targets: TargetSet,
        changed_at: i64,
    },
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
            Self::Touch { targets, .. } => HEADER + targets.estimated_bytes(),
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
            Self::Touch { targets, .. }
            | Self::Tags { targets, .. }
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

    fn push(&mut self, mut operation: DirtyOperation) {
        // Coalesce repeated edits in the active sequence. This is especially
        // important for UI toggles and create -> rename -> share journeys.
        if let DirtyOperation::Touch { targets, .. } = &operation
            && let Some(index) = self.operations.iter().rposition(|candidate| {
                matches!(candidate, DirtyOperation::Touch { targets: old, .. } if old == targets)
            })
        {
            let previous = self.operations.remove(index);
            let DirtyOperation::Touch {
                changed_at: previous,
                ..
            } = previous
            else {
                unreachable!();
            };
            let DirtyOperation::Touch { changed_at, .. } = &mut operation else {
                unreachable!();
            };
            *changed_at = (*changed_at).max(previous);
            self.recount_bytes();
        }
        match &operation {
            DirtyOperation::Touch { .. } => {}
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
            DirtyOperation::Touch {
                targets: pending, ..
            }
            | DirtyOperation::Tags {
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

    /// Retire target-centric edits that were folded into a direct media
    /// publication without discarding the same edit for other targets. Album
    /// upserts are also retired when the publication persisted a freshly
    /// aggregated copy of that album.
    fn cancel_published_media(
        &mut self,
        targets: &TargetSet,
        published_album_ids: &BTreeSet<ArrayString<64>>,
    ) {
        self.operations.retain_mut(|operation| match operation {
            DirtyOperation::Touch {
                targets: pending, ..
            }
            | DirtyOperation::Tags {
                targets: pending, ..
            }
            | DirtyOperation::Flags {
                targets: pending, ..
            }
            | DirtyOperation::Albums {
                targets: pending, ..
            } => {
                pending.subtract(targets);
                !pending.is_empty()
            }
            DirtyOperation::Description { target, .. } => !targets.contains(*target),
            DirtyOperation::AlbumCreate(album) | DirtyOperation::AlbumReplace(album) => {
                !published_album_ids.contains(&album.object.id)
            }
            // A deleted album has no aggregate for media publication to
            // persist, so its structural deletion must remain queued.
            DirtyOperation::AlbumDelete(_) => true,
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
    last_flush_records: usize,
    last_flush_unique_records: usize,
    last_flush_chunks: usize,
    flush_records_per_second: Option<f64>,
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
            last_flush_records: 0,
            last_flush_unique_records: 0,
            last_flush_chunks: 0,
            flush_records_per_second: None,
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
    pub flush_chunk_records: usize,
    pub pending_operations: usize,
    pub pending_records: usize,
    pub active_records: usize,
    pub flushing_records: usize,
    pub pending_bytes: usize,
    pub flushing: bool,
    pub oldest_pending_age_ms: Option<u64>,
    pub last_flush_time_ms: Option<u64>,
    pub last_flush_duration_ms: Option<u64>,
    pub last_flush_records: usize,
    pub last_flush_unique_records: usize,
    pub last_flush_chunks: usize,
    pub flush_records_per_second: Option<f64>,
    pub estimated_drain_ms: Option<u64>,
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
        let pending_records = active_records + flushing_records;
        let estimated_drain_ms = state.flush_records_per_second.and_then(|rate| {
            (rate > 0.0).then(|| {
                ((pending_records as f64 / rate) * 1_000.0)
                    .ceil()
                    .min(u64::MAX as f64) as u64
            })
        });
        WriteBehindStatus {
            flush_chunk_records: FLUSH_CHUNK_SIZE,
            pending_operations: active_operations + flushing_operations,
            pending_records,
            active_records,
            flushing_records,
            pending_bytes: state.pending_bytes(),
            flushing: state.flushing.is_some(),
            oldest_pending_age_ms: oldest
                .map(|created| u64::try_from(created.elapsed().as_millis()).unwrap_or(u64::MAX)),
            last_flush_time_ms: state.last_flush_time_ms,
            last_flush_duration_ms: state.last_flush_duration_ms,
            last_flush_records: state.last_flush_records,
            last_flush_unique_records: state.last_flush_unique_records,
            last_flush_chunks: state.last_flush_chunks,
            flush_records_per_second: state.flush_records_per_second,
            estimated_drain_ms,
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

    /// Reconcile edits already materialized by a selective media commit. In
    /// contrast to `cancel_targets`, album add/remove sets stay intact for all
    /// remaining targets in the same write-behind operation.
    pub fn cancel_published_media(
        &self,
        targets: &TargetSet,
        published_album_ids: &BTreeSet<ArrayString<64>>,
    ) {
        let mut state = self.state.lock().unwrap();
        state
            .active
            .cancel_published_media(targets, published_album_ids);
        if state.flushing.is_some() {
            state.cancelled_flushing_slots.extend(targets.iter());
            state
                .cancelled_flushing_albums
                .extend(published_album_ids.iter().copied());
            if let Some(flushing) = &mut state.flushing {
                Arc::make_mut(flushing).cancel_published_media(targets, published_album_ids);
            }
        }
        drop(state);
        self.capacity_changed.notify_waiters();
    }

    fn retain_uncancelled_slots(&self, slots: &mut Vec<crate::public::db::tree::state::SlotRef>) {
        let state = self.state.lock().unwrap();
        slots.retain(|slot_ref| !state.cancelled_flushing_slots.contains(slot_ref));
    }

    fn is_flushing_album_cancelled(&self, album_id: &ArrayString<64>) -> bool {
        self.state
            .lock()
            .unwrap()
            .cancelled_flushing_albums
            .contains(album_id)
    }

    fn retain_uncancelled_albums(&self, album_ids: &mut Vec<ArrayString<64>>) {
        let state = self.state.lock().unwrap();
        album_ids.retain(|album_id| !state.cancelled_flushing_albums.contains(album_id));
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

    fn complete(&self, sequence: u64, started: Instant, result: &anyhow::Result<FlushStats>) {
        let mut state = self.state.lock().unwrap();
        match result {
            Ok(flush_stats) => {
                if state
                    .flushing
                    .as_ref()
                    .is_some_and(|batch| batch.sequence == sequence)
                {
                    state.flushing = None;
                }
                state.last_flush_time_ms = Some(unix_time_ms());
                let elapsed = started.elapsed();
                state.last_flush_duration_ms =
                    Some(u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX));
                state.last_flush_records = flush_stats.records;
                state.last_flush_unique_records = flush_stats.unique_records;
                state.last_flush_chunks = flush_stats.chunks;
                if flush_stats.records > 0 && !elapsed.is_zero() {
                    let rate = flush_stats.records as f64 / elapsed.as_secs_f64();
                    state.flush_records_per_second =
                        Some(state.flush_records_per_second.map_or(rate, |previous| {
                            previous * (1.0 - FLUSH_RATE_EWMA_ALPHA) + rate * FLUSH_RATE_EWMA_ALPHA
                        }));
                }
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

#[derive(Debug, Clone, Copy, Default)]
struct FlushStats {
    /// Logical record touches retired from the dirty queue. This remains
    /// comparable with `pending_records` for drain-time estimation.
    records: usize,
    /// Durable records actually decoded and written. Target-centric batches
    /// can retire several logical touches with one materialization.
    unique_records: usize,
    chunks: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct DetailedTargetTiming {
    decode: Duration,
    overlay: Duration,
    encode_insert: Duration,
    commit: Duration,
}

impl DetailedTargetTiming {
    fn add_write(&mut self, timing: crate::storage::store::RecordWriteTiming) {
        self.decode = self.decode.saturating_add(timing.decode);
        self.encode_insert = self.encode_insert.saturating_add(timing.encode_insert);
        self.commit = self.commit.saturating_add(timing.commit);
    }
}

fn persist_batch(batch: &DirtyBatch) -> anyhow::Result<FlushStats> {
    let batch_started = Instant::now();
    let mut stats = FlushStats {
        records: batch.records(),
        ..FlushStats::default()
    };
    let structural_album_ids = batch
        .operations
        .iter()
        .filter_map(|operation| match operation {
            DirtyOperation::AlbumCreate(album) | DirtyOperation::AlbumReplace(album) => {
                Some(album.object.id)
            }
            DirtyOperation::AlbumDelete(album_id) => Some(*album_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    // A pending album must exist durably before media membership chunks can
    // reference it. The final album pass below reapplies the complete sequence
    // after media records, preserving aggregate and metadata ordering.
    let create_started = Instant::now();
    persist_album_creates(batch, &mut stats)?;
    crate::perf_timing!(
        "write_behind.flush.album_create",
        create_started,
        "Flush pending album records"
    );

    let targets_started = Instant::now();
    let target_timing = persist_target_records(batch, &structural_album_ids, &mut stats)?;
    if crate::performance::detailed_timing_enabled() {
        crate::perf_duration!(
            "write_behind.flush.targets.decode",
            target_timing.decode,
            "Decode target records"
        );
        crate::perf_duration!(
            "write_behind.flush.targets.overlay",
            target_timing.overlay,
            "Apply target overlays"
        );
        crate::perf_duration!(
            "write_behind.flush.targets.encode_insert",
            target_timing.encode_insert,
            "Encode and insert target records"
        );
        crate::perf_duration!(
            "write_behind.flush.targets.commit",
            target_timing.commit,
            "Commit target record transactions"
        );
    }
    crate::perf_timing!(
        "write_behind.flush.targets",
        targets_started,
        "Flush target-centric metadata records"
    );

    // Structural album records are finalized after member records. Applying
    // the whole operation list to each album preserves exact operation order
    // even when flags/tags surround an AlbumReplace in the same sequence.
    let albums_started = Instant::now();
    persist_structural_albums(batch, &structural_album_ids, &mut stats)?;
    crate::perf_timing!(
        "write_behind.flush.album_finalize",
        albums_started,
        "Finalize structural album records"
    );

    crate::perf_timing!(
        "write_behind.flush.batch",
        batch_started,
        "Flush dirty batch sequence {}",
        batch.sequence
    );
    log::info!(
        operation = "write_behind.flush.batch_work",
        records = stats.records,
        unique_records = stats.unique_records,
        chunks = stats.chunks;
        "Retired {} record touches by materializing {} records in {} chunks",
        stats.records,
        stats.unique_records,
        stats.chunks
    );
    Ok(stats)
}

fn persist_album_creates(batch: &DirtyBatch, stats: &mut FlushStats) -> anyhow::Result<()> {
    let albums = batch
        .operations
        .iter()
        .filter_map(|operation| match operation {
            DirtyOperation::AlbumCreate(album) => Some(album),
            _ => None,
        })
        .collect::<Vec<_>>();
    for album_chunk in albums.chunks(FLUSH_CHUNK_SIZE) {
        let _persistence_guard = TREE.persistence_lock.lock().unwrap();
        let mut chunk = album_chunk.to_vec();
        chunk.retain(|album| !WRITE_BEHIND.is_flushing_album_cancelled(&album.object.id));
        if chunk.is_empty() {
            continue;
        }
        before_flush_commit()?;
        TREE.store.write(|writer| {
            for album in &chunk {
                writer.insert_at_owned(
                    album.object.id.as_str(),
                    AbstractData::Album((**album).clone()),
                )?;
            }
            Ok::<(), anyhow::Error>(())
        })?;
        stats.unique_records = stats.unique_records.saturating_add(chunk.len());
        stats.chunks = stats.chunks.saturating_add(1);
    }
    Ok(())
}

fn persist_target_records(
    batch: &DirtyBatch,
    structural_album_ids: &BTreeSet<ArrayString<64>>,
    stats: &mut FlushStats,
) -> anyhow::Result<DetailedTargetTiming> {
    let detailed = crate::performance::detailed_timing_enabled();
    let mut timing = DetailedTargetTiming::default();
    let universe = TREE.state.read().unwrap().arena.capacity();
    let mut targets = TargetSet::default();
    for operation in &batch.operations {
        match operation {
            DirtyOperation::Touch {
                targets: operation_targets,
                ..
            }
            | DirtyOperation::Tags {
                targets: operation_targets,
                ..
            }
            | DirtyOperation::Albums {
                targets: operation_targets,
                ..
            }
            | DirtyOperation::Flags {
                targets: operation_targets,
                ..
            } => targets.union(operation_targets, universe),
            DirtyOperation::Description { target, .. } => {
                targets.union(&TargetSet::from_slot_refs([*target], universe), universe)
            }
            DirtyOperation::AlbumCreate(_)
            | DirtyOperation::AlbumReplace(_)
            | DirtyOperation::AlbumDelete(_) => {}
        }
    }

    let mut slots = targets.iter();
    loop {
        let mut slot_chunk = slots.by_ref().take(FLUSH_CHUNK_SIZE).collect::<Vec<_>>();
        if slot_chunk.is_empty() {
            break;
        }
        let _persistence_guard = TREE.persistence_lock.lock().unwrap();
        WRITE_BEHIND.retain_uncancelled_slots(&mut slot_chunk);
        let records = {
            let tree_state = TREE.state.read().unwrap();
            slot_chunk
                .into_iter()
                .filter_map(|slot_ref| {
                    let id = tree_state.get(slot_ref)?.id;
                    (!structural_album_ids.contains(&id)).then_some((slot_ref, id))
                })
                .collect::<Vec<_>>()
        };
        if records.is_empty() {
            continue;
        }
        before_flush_commit()?;
        if detailed {
            let (overlay, write_timing) = TREE
                .store
                .write_profiled(|writer| persist_target_chunk_profiled(writer, &records, batch))?;
            timing.overlay = timing.overlay.saturating_add(overlay);
            timing.add_write(write_timing);
        } else {
            TREE.store
                .write(|writer| persist_target_chunk(writer, &records, batch))?;
        }
        stats.unique_records = stats.unique_records.saturating_add(records.len());
        stats.chunks = stats.chunks.saturating_add(1);
    }
    Ok(timing)
}

fn persist_target_chunk(
    writer: &mut RecordWriter<'_>,
    records: &[(crate::public::db::tree::state::SlotRef, ArrayString<64>)],
    batch: &DirtyBatch,
) -> anyhow::Result<()> {
    for (slot_ref, id) in records {
        let mut record = writer.get_v6(id.as_str())?;
        apply_v6_overlay(&mut record, Some(*slot_ref), id.as_str(), &batch.operations);
        if let Some(record) = record {
            writer.insert_v6_at(id.as_str(), record)?;
        }
    }
    Ok(())
}

fn persist_target_chunk_profiled(
    writer: &mut RecordWriter<'_>,
    records: &[(crate::public::db::tree::state::SlotRef, ArrayString<64>)],
    batch: &DirtyBatch,
) -> anyhow::Result<Duration> {
    let mut overlay = Duration::ZERO;
    for (slot_ref, id) in records {
        let mut record = writer.get_v6_profiled(id.as_str())?;
        let overlay_started = Instant::now();
        apply_v6_overlay(&mut record, Some(*slot_ref), id.as_str(), &batch.operations);
        overlay = overlay.saturating_add(overlay_started.elapsed());
        if let Some(record) = record {
            writer.insert_v6_at_profiled(id.as_str(), record)?;
        }
    }
    Ok(overlay)
}

fn persist_structural_albums(
    batch: &DirtyBatch,
    structural_album_ids: &BTreeSet<ArrayString<64>>,
    stats: &mut FlushStats,
) -> anyhow::Result<()> {
    let album_ids = structural_album_ids.iter().copied().collect::<Vec<_>>();
    for album_chunk in album_ids.chunks(FLUSH_CHUNK_SIZE) {
        let _persistence_guard = TREE.persistence_lock.lock().unwrap();
        let mut album_chunk = album_chunk.to_vec();
        WRITE_BEHIND.retain_uncancelled_albums(&mut album_chunk);
        if album_chunk.is_empty() {
            continue;
        }
        let slots = {
            let tree_state = TREE.state.read().unwrap();
            album_chunk
                .iter()
                .map(|album_id| (*album_id, tree_state.find(album_id.as_str())))
                .collect::<Vec<_>>()
        };
        before_flush_commit()?;
        TREE.store.write(|writer| {
            for (album_id, slot_ref) in &slots {
                let mut record = writer.get_v6(album_id.as_str())?;
                apply_v6_overlay(&mut record, *slot_ref, album_id.as_str(), &batch.operations);
                if let Some(record) = record {
                    writer.insert_v6_at(album_id.as_str(), record)?;
                } else {
                    writer.remove(album_id.as_str())?;
                }
            }
            Ok::<(), anyhow::Error>(())
        })?;
        stats.unique_records = stats.unique_records.saturating_add(slots.len());
        stats.chunks = stats.chunks.saturating_add(1);
    }
    Ok(())
}

fn apply_record_operation(data: &mut AbstractData, operation: &DirtyOperation) {
    match operation {
        DirtyOperation::Touch { changed_at, .. } => data.touch_update_at(*changed_at),
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

fn apply_v6_record_operation(data: &mut V6AbstractData, operation: &DirtyOperation) {
    match operation {
        DirtyOperation::Touch { changed_at, .. } => {
            let object = data.object_mut();
            object.update_at = object.update_at.max(*changed_at);
            crate::public::structure::object::observe_mutation_timestamp(object.update_at);
        }
        DirtyOperation::Tags { add, remove, .. } => {
            let tags = &mut data.object_mut().tags;
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
            let object = data.object_mut();
            if let Some(value) = favorite {
                object.is_favorite = *value;
            }
            if let Some(value) = archived {
                object.is_archived = *value;
            }
            if let Some(value) = trashed {
                object.is_trashed = *value;
            }
        }
        DirtyOperation::Description { value, .. } => {
            data.object_mut().description.clone_from(value);
        }
        DirtyOperation::AlbumCreate(_)
        | DirtyOperation::AlbumReplace(_)
        | DirtyOperation::AlbumDelete(_) => {}
    }
}

fn apply_v6_overlay(
    record: &mut Option<V6AbstractData>,
    slot_ref: Option<crate::public::db::tree::state::SlotRef>,
    id: &str,
    operations: &[DirtyOperation],
) {
    for operation in operations {
        match operation {
            DirtyOperation::AlbumCreate(album) | DirtyOperation::AlbumReplace(album)
                if album.object.id.as_str() == id =>
            {
                *record = Some(V6AbstractData::from(&AbstractData::Album(album.clone())));
            }
            DirtyOperation::AlbumDelete(album_id) if album_id.as_str() == id => {
                *record = None;
            }
            DirtyOperation::Touch { targets, .. }
            | DirtyOperation::Tags { targets, .. }
            | DirtyOperation::Albums { targets, .. }
            | DirtyOperation::Flags { targets, .. }
                if slot_ref.is_some_and(|slot_ref| targets.contains(slot_ref)) =>
            {
                if let Some(data) = record {
                    apply_v6_record_operation(data, operation);
                }
            }
            DirtyOperation::Description { target, .. } if slot_ref == Some(*target) => {
                if let Some(data) = record {
                    apply_v6_record_operation(data, operation);
                }
            }
            _ => {}
        }
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
            DirtyOperation::Touch { targets, .. }
            | DirtyOperation::Tags { targets, .. }
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
    use crate::public::structure::image::{ImageCombined, ImageMetadata};
    use crate::public::structure::object::{ObjectSchema, ObjectType};

    fn targets(values: impl IntoIterator<Item = u32>) -> TargetSet {
        TargetSet::from_slot_refs(values.into_iter().map(|value| SlotRef::new(value, 1)), 64)
    }

    fn image(update_at: i64, cache_version: u32) -> AbstractData {
        let id = ArrayString::<64>::from(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let mut object = ObjectSchema::new(id, ObjectType::Image);
        object.update_at = update_at;
        object.cache_version = cache_version;
        AbstractData::Image(ImageCombined {
            object,
            metadata: ImageMetadata::new(id, 10, 20, 30, "jpg".to_owned()),
        })
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
    fn inverse_metadata_edits_keep_the_latest_touch() {
        let targets = targets([1, 2]);
        let mut batch = DirtyBatch::new(1);
        batch.push(DirtyOperation::Tags {
            targets: targets.clone(),
            add: BTreeSet::from(["marker".to_owned()]),
            remove: BTreeSet::new(),
        });
        batch.push(DirtyOperation::Touch {
            targets: targets.clone(),
            changed_at: 100,
        });
        batch.push(DirtyOperation::Tags {
            targets: targets.clone(),
            add: BTreeSet::new(),
            remove: BTreeSet::from(["marker".to_owned()]),
        });
        batch.push(DirtyOperation::Touch {
            targets,
            changed_at: 101,
        });

        assert_eq!(batch.operations.len(), 1);
        assert!(matches!(
            &batch.operations[0],
            DirtyOperation::Touch {
                changed_at: 101,
                ..
            }
        ));
    }

    #[test]
    fn coalesced_touch_stays_after_interleaved_album_replacement() {
        let target_set = targets([1]);
        let album_id = ArrayString::<64>::from("touch-order-album").unwrap();
        let mut album = match Album::new(album_id, None).into_abstract_data() {
            AbstractData::Album(album) => album,
            _ => unreachable!(),
        };
        album.object.update_at = 10;
        let slot = SlotRef::new(1, 1);
        let mut batch = DirtyBatch::new(1);
        batch.push(DirtyOperation::Touch {
            targets: target_set.clone(),
            changed_at: 100,
        });
        batch.push(DirtyOperation::AlbumReplace(album.clone()));
        batch.push(DirtyOperation::Touch {
            targets: target_set,
            changed_at: 101,
        });

        assert!(matches!(
            batch.operations.last(),
            Some(DirtyOperation::Touch {
                changed_at: 101,
                ..
            })
        ));
        let mut record = Some(AbstractData::Album(album));
        apply_overlay(
            &mut record,
            Some(slot),
            album_id.as_str(),
            &batch.operations,
        );
        let Some(AbstractData::Album(record)) = record else {
            unreachable!();
        };
        assert_eq!(record.object.update_at, 101);
    }

    #[test]
    fn touch_overlay_is_stable_and_does_not_change_thumbnail_version() {
        let slot = SlotRef::new(1, 1);
        let target_set = TargetSet::from_slot_refs([slot], 64);
        let operation = DirtyOperation::Touch {
            targets: target_set,
            changed_at: 100,
        };
        let base = image(10, 7);
        let id = base.hash().to_string();

        let mut first_read = Some(base.clone());
        apply_overlay(
            &mut first_read,
            Some(slot),
            &id,
            std::slice::from_ref(&operation),
        );
        let mut second_read = Some(base.clone());
        apply_overlay(
            &mut second_read,
            Some(slot),
            &id,
            std::slice::from_ref(&operation),
        );

        let first_read = first_read.unwrap();
        let second_read = second_read.unwrap();
        assert_eq!(first_read.cache_version(), 7);
        assert_eq!(second_read.cache_version(), 7);
        let first_update_at = match first_read {
            AbstractData::Image(image) => image.object.update_at,
            _ => unreachable!(),
        };
        let second_update_at = match second_read {
            AbstractData::Image(image) => image.object.update_at,
            _ => unreachable!(),
        };
        assert_eq!(first_update_at, 100);
        assert_eq!(second_update_at, 100);

        let mut stored = V6AbstractData::from(&base);
        apply_v6_record_operation(&mut stored, &operation);
        let durable = stored.into_domain().unwrap();
        assert_eq!(durable.cache_version(), 7);
        let AbstractData::Image(durable) = durable else {
            unreachable!();
        };
        assert_eq!(durable.object.update_at, 100);

        let mut newer_record = Some(image(200, 7));
        apply_overlay(
            &mut newer_record,
            Some(slot),
            &id,
            std::slice::from_ref(&operation),
        );
        let Some(AbstractData::Image(newer_record)) = newer_record else {
            unreachable!();
        };
        assert_eq!(newer_record.object.update_at, 200);
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

    #[test]
    fn status_reports_flush_work_and_estimated_drain() {
        let store = DirtyDeltaStore::new();
        store
            .state
            .lock()
            .unwrap()
            .active
            .push(DirtyOperation::Flags {
                targets: targets(0..100),
                favorite: Some(true),
                archived: None,
                trashed: None,
            });
        let started = Instant::now() - Duration::from_secs(1);
        store.complete(
            0,
            started,
            &Ok(FlushStats {
                records: 200,
                unique_records: 100,
                chunks: 2,
            }),
        );
        let status = store.status();
        assert_eq!(status.active_records, 100);
        assert_eq!(status.flushing_records, 0);
        assert_eq!(status.last_flush_records, 200);
        assert_eq!(status.last_flush_unique_records, 100);
        assert_eq!(status.last_flush_chunks, 2);
        assert!(
            status
                .flush_records_per_second
                .is_some_and(|rate| rate > 190.0)
        );
        assert!(status.estimated_drain_ms.is_some_and(|value| value >= 490));
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

        let mut stored = V6AbstractData::from(&data);
        apply_v6_record_operation(
            &mut stored,
            &DirtyOperation::Tags {
                targets: target_set.clone(),
                add: BTreeSet::from(["new-tag".to_owned()]),
                remove: BTreeSet::new(),
            },
        );
        apply_v6_record_operation(
            &mut stored,
            &DirtyOperation::Flags {
                targets: target_set,
                favorite: Some(true),
                archived: Some(true),
                trashed: Some(false),
            },
        );
        apply_v6_record_operation(
            &mut stored,
            &DirtyOperation::Description {
                target: slot,
                value: Some("patched".to_owned()),
            },
        );

        let data = stored.into_domain().unwrap();
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

    #[test]
    fn target_centric_overlay_preserves_operation_order_around_album_replace() {
        let album_id = ArrayString::<64>::from("ordered-album").unwrap();
        let mut album = match Album::new(album_id, Some("before".to_owned())).into_abstract_data() {
            AbstractData::Album(album) => album,
            _ => unreachable!(),
        };
        album.object.is_favorite = false;
        album.object.is_archived = false;
        let slot = SlotRef::new(5, 1);
        let targets = TargetSet::from_slot_refs([slot], 64);
        let mut replacement = album.clone();
        replacement.metadata.title = Some("replacement".to_owned());
        let operations = vec![
            DirtyOperation::Flags {
                targets: targets.clone(),
                favorite: Some(true),
                archived: None,
                trashed: None,
            },
            DirtyOperation::AlbumReplace(replacement),
            DirtyOperation::Flags {
                targets: targets.clone(),
                favorite: None,
                archived: Some(true),
                trashed: None,
            },
            DirtyOperation::Tags {
                targets,
                add: BTreeSet::from(["after-replace".to_owned()]),
                remove: BTreeSet::new(),
            },
        ];
        let mut record = Some(AbstractData::Album(album));
        apply_overlay(&mut record, Some(slot), album_id.as_str(), &operations);
        let AbstractData::Album(result) = record.unwrap() else {
            unreachable!();
        };
        assert!(!result.object.is_favorite);
        assert!(result.object.is_archived);
        assert_eq!(result.metadata.title.as_deref(), Some("replacement"));
        assert!(result.object.tags.contains("after-replace"));
    }

    #[test]
    fn media_publication_only_retires_the_committed_target_from_album_edits() {
        let album_id = ArrayString::<64>::from("shared-album-edit").unwrap();
        let deleted_album_id = ArrayString::<64>::from("deleted-album").unwrap();
        let album = match Album::new(album_id, Some("Shared".to_owned())).into_abstract_data() {
            AbstractData::Album(album) => album,
            _ => unreachable!(),
        };
        let committed = SlotRef::new(1, 1);
        let still_pending = SlotRef::new(2, 1);
        let mut batch = DirtyBatch::new(1);
        batch.push(DirtyOperation::Albums {
            targets: TargetSet::from_slot_refs([committed, still_pending], 64),
            add: BTreeSet::from([album_id]),
            remove: BTreeSet::new(),
        });
        batch.push(DirtyOperation::AlbumReplace(album));
        batch.push(DirtyOperation::AlbumDelete(deleted_album_id));

        batch.cancel_published_media(
            &TargetSet::from_slot_refs([committed], 64),
            &BTreeSet::from([album_id]),
        );

        assert!(matches!(
            &batch.operations[0],
            DirtyOperation::Albums { targets, add, .. }
                if !targets.contains(committed)
                    && targets.contains(still_pending)
                    && add.contains(&album_id)
        ));
        assert!(!batch.operations.iter().any(
            |operation| matches!(operation, DirtyOperation::AlbumReplace(album) if album.object.id == album_id)
        ));
        assert!(batch.operations.iter().any(
            |operation| matches!(operation, DirtyOperation::AlbumDelete(id) if *id == deleted_album_id)
        ));
    }

    #[cfg(feature = "performance-test")]
    #[test]
    #[ignore = "targeted Redb flush chunk throughput matrix"]
    fn flush_chunk_matrix_selects_the_smallest_material_improvement() {
        use crate::storage::DataStore;

        const RECORDS: usize = 100_000;
        const CANDIDATES: [usize; 4] = [4_096, 8_192, 16_384, 32_768];
        const MAX_CHUNK_WORKING_BYTES: usize = 64 * 1024 * 1024;

        fn patch_tag(
            store: &DataStore,
            ids: &[ArrayString<64>],
            chunk_size: usize,
            tag: &str,
            add: bool,
        ) {
            for chunk in ids.chunks(chunk_size) {
                store
                    .write(|writer| {
                        for id in chunk {
                            let mut data = writer.get(id.as_str())?.unwrap().into_value();
                            if add {
                                data.tag_mut().insert(tag.to_owned());
                            } else {
                                data.tag_mut().remove(tag);
                            }
                            writer.insert_at(id.as_str(), &data)?;
                        }
                        Ok::<(), anyhow::Error>(())
                    })
                    .unwrap();
            }
        }

        let directory = tempfile::tempdir().unwrap();
        let store = DataStore::initialize_empty(&directory.path().join("matrix.redb")).unwrap();
        let mut ids = Vec::with_capacity(RECORDS);
        for start in (0..RECORDS).step_by(FLUSH_CHUNK_SIZE) {
            let end = (start + FLUSH_CHUNK_SIZE).min(RECORDS);
            let records = (start..end)
                .map(|index| AbstractData::generate_performance_data(index as u64, 20_260_718))
                .collect::<Vec<_>>();
            ids.extend(records.iter().map(AbstractData::hash));
            store
                .write(|writer| {
                    for record in &records {
                        writer.insert(record)?;
                    }
                    Ok::<(), anyhow::Error>(())
                })
                .unwrap();
        }

        patch_tag(&store, &ids, FLUSH_CHUNK_SIZE, "flush-matrix-warmup", true);
        patch_tag(&store, &ids, FLUSH_CHUNK_SIZE, "flush-matrix-warmup", false);

        let mut rates = CANDIDATES
            .into_iter()
            .map(|candidate| (candidate, Vec::<f64>::new()))
            .collect::<std::collections::BTreeMap<_, _>>();
        for (round, candidates) in [
            CANDIDATES.to_vec(),
            CANDIDATES.into_iter().rev().collect::<Vec<_>>(),
        ]
        .into_iter()
        .enumerate()
        {
            for candidate in candidates {
                let tag = format!("flush-matrix-{round}-{candidate}");
                let started = Instant::now();
                patch_tag(&store, &ids, candidate, &tag, true);
                patch_tag(&store, &ids, candidate, &tag, false);
                let elapsed = started.elapsed();
                let rate = (RECORDS * 2) as f64 / elapsed.as_secs_f64();
                rates.get_mut(&candidate).unwrap().push(rate);
                println!(
                    "flush-matrix chunk={candidate} round={round} duration_ms={} records_per_second={rate:.2}",
                    elapsed.as_millis()
                );
            }
        }

        let averages = rates
            .iter()
            .map(|(candidate, values)| {
                let average = values.iter().sum::<f64>() / values.len() as f64;
                (*candidate, average)
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let baseline = averages[&FLUSH_CHUNK_SIZE];
        let recommended = CANDIDATES.into_iter().find(|candidate| {
            let estimated_working_bytes = candidate
                .saturating_mul(
                    std::mem::size_of::<SlotRef>() + std::mem::size_of::<ArrayString<64>>(),
                )
                .saturating_add(1024 * 1024);
            estimated_working_bytes <= MAX_CHUNK_WORKING_BYTES
                && averages[candidate] >= baseline * 1.15
        });
        for (candidate, average) in &averages {
            println!(
                "flush-matrix-summary chunk={candidate} records_per_second={average:.2} relative={:.3}",
                average / baseline
            );
        }
        println!(
            "flush-matrix-recommended={}",
            recommended.unwrap_or(FLUSH_CHUNK_SIZE)
        );
    }
}
