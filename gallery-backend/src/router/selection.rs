use rocket::serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::operations::open_db::open_tree_snapshot_table;
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::{TargetSet, TargetSetBuilder, TreeState};
use crate::public::db::tree_snapshot::read_tree_snapshot::PinnedSnapshotView;
use crate::public::error::{AppError, ErrorKind};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SelectionDescriptor {
    Explicit { indices: Vec<u32> },
    AllExcept { excluded_indices: Vec<u32> },
}

impl SelectionDescriptor {
    pub fn explicit(indices: Vec<u32>) -> Self {
        Self::Explicit { indices }
    }
}

#[derive(Debug)]
pub struct ResolvedSelection {
    pub targets: TargetSet,
    pub len: usize,
    pub identity_epoch: u64,
    pub selection_epoch: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum NormalizedSelection {
    ExplicitAll,
    Explicit(Vec<u32>),
    AllExceptNone,
    AllExcept(Vec<u32>),
}

#[derive(Debug)]
struct IndexMembership {
    words: Vec<u64>,
    len: usize,
}

impl IndexMembership {
    fn with_universe(universe: usize) -> Self {
        Self {
            words: vec![0; universe.div_ceil(u64::BITS as usize)],
            len: 0,
        }
    }

    fn insert(&mut self, index: u32) {
        let index = index as usize;
        let word = index / u64::BITS as usize;
        let mask = 1_u64 << (index % u64::BITS as usize);
        if self.words[word] & mask == 0 {
            self.words[word] |= mask;
            self.len += 1;
        }
    }

    #[cfg(test)]
    fn estimated_bytes(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>()
    }
}

fn normalize_selection(
    selection: SelectionDescriptor,
    snapshot_len: usize,
) -> Result<NormalizedSelection, AppError> {
    let (mut values, explicit) = match selection {
        SelectionDescriptor::Explicit { indices } => (indices, true),
        SelectionDescriptor::AllExcept { excluded_indices } => (excluded_indices, false),
    };
    let mut membership = IndexMembership::with_universe(snapshot_len);
    for index in values.iter().copied() {
        if index as usize >= snapshot_len {
            return Err(AppError::new(
                ErrorKind::Conflict,
                "selection contains an out-of-range index",
            ));
        }
        membership.insert(index);
    }
    if explicit && membership.len == snapshot_len {
        return Ok(NormalizedSelection::ExplicitAll);
    }
    if !explicit && membership.len == 0 {
        return Ok(NormalizedSelection::AllExceptNone);
    }
    let unique_len = membership.len;
    drop(membership);
    values.sort_unstable();
    values.dedup();
    debug_assert_eq!(values.len(), unique_len);
    Ok(if explicit {
        NormalizedSelection::Explicit(values)
    } else {
        NormalizedSelection::AllExcept(values)
    })
}

fn resolve_indices(
    snapshot: &PinnedSnapshotView<'_>,
    indices: &[u32],
    state: &TreeState,
    validate_identities: bool,
    timestamp: i64,
) -> Result<TargetSet, AppError> {
    let mut targets = TargetSetBuilder::default();
    for index in indices {
        let slot_ref = snapshot.slot_ref(*index as usize).map_err(|error| {
            AppError::from_err(
                ErrorKind::Conflict,
                anyhow::anyhow!("stale selection index {index} for {timestamp}: {error}"),
            )
        })?;
        if validate_identities && state.get(slot_ref).is_none() {
            return Err(AppError::new(
                ErrorKind::Conflict,
                format!("selection index {index} has a stale generation"),
            ));
        }
        targets.insert(slot_ref);
    }
    Ok(targets.finish(state.arena.capacity()))
}

fn resolve_all_except(
    snapshot: &PinnedSnapshotView<'_>,
    excluded: &[u32],
    state: &TreeState,
    timestamp: i64,
) -> Result<TargetSet, AppError> {
    let mut targets = TargetSetBuilder::default();
    let mut excluded_cursor = 0;
    for index in 0..snapshot.len() {
        if excluded.get(excluded_cursor).copied() == u32::try_from(index).ok() {
            excluded_cursor += 1;
            continue;
        }
        let slot_ref = snapshot.slot_ref(index).map_err(|error| {
            AppError::from_err(
                ErrorKind::Conflict,
                anyhow::anyhow!("stale selection index {index} for {timestamp}: {error}"),
            )
        })?;
        if state.get(slot_ref).is_none() {
            return Err(AppError::new(
                ErrorKind::Conflict,
                format!("selection index {index} has a stale generation"),
            ));
        }
        targets.insert(slot_ref);
    }
    Ok(targets.finish(state.arena.capacity()))
}

fn snapshot_target_set(
    snapshot: &PinnedSnapshotView<'_>,
    timestamp: i64,
) -> Result<TargetSet, AppError> {
    snapshot.target_set().map_err(|error| {
        AppError::from_err(
            ErrorKind::Conflict,
            anyhow::anyhow!("invalid selection target bitmap for {timestamp}: {error}"),
        )
    })
}

// Owning this small view keeps the Redb value guard visibly scoped to the
// complete bulk resolution operation.
#[allow(clippy::needless_pass_by_value)]
fn resolve_pinned_selection(
    snapshot: PinnedSnapshotView<'_>,
    timestamp: i64,
    selection: SelectionDescriptor,
    state: &TreeState,
) -> Result<TargetSet, AppError> {
    let normalized = normalize_selection(selection, snapshot.len())?;
    if snapshot.identity_epoch() != state.identity_epoch() {
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection snapshot belongs to a different tree identity epoch",
        ));
    }
    let validate_identities = snapshot.selection_epoch() != state.selection_epoch();

    match normalized {
        NormalizedSelection::ExplicitAll | NormalizedSelection::AllExceptNone
            if !validate_identities =>
        {
            snapshot_target_set(&snapshot, timestamp)
        }
        NormalizedSelection::Explicit(indices) => {
            resolve_indices(&snapshot, &indices, state, validate_identities, timestamp)
        }
        NormalizedSelection::AllExcept(excluded) if !validate_identities => {
            let mut targets = snapshot_target_set(&snapshot, timestamp)?;
            let excluded_targets = resolve_indices(&snapshot, &excluded, state, false, timestamp)?;
            targets.subtract(&excluded_targets);
            Ok(targets)
        }
        NormalizedSelection::ExplicitAll | NormalizedSelection::AllExceptNone => {
            resolve_all_except(&snapshot, &[], state, timestamp)
        }
        NormalizedSelection::AllExcept(excluded) => {
            resolve_all_except(&snapshot, &excluded, state, timestamp)
        }
    }
}

pub fn resolved_selection_is_current(
    state: &TreeState,
    identity_epoch: u64,
    selection_epoch: u64,
    targets: &TargetSet,
) -> bool {
    state.identity_epoch() == identity_epoch
        && (state.selection_epoch() == selection_epoch || targets.is_current(state))
}

/// Resolve a UI selection against its immutable tree snapshot and validate the
/// complete generational identity set before any mutation is published.
pub fn resolve_selection(
    timestamp: i64,
    selection: SelectionDescriptor,
) -> Result<ResolvedSelection, AppError> {
    let started = Instant::now();
    let snapshot = open_tree_snapshot_table(timestamp).map_err(|error| {
        AppError::from_err(
            ErrorKind::Conflict,
            anyhow::anyhow!("stale selection snapshot {timestamp}: {error}"),
        )
    })?;

    let state = TREE
        .state
        .read()
        .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
    let targets = snapshot
        .with_pinned_view(|snapshot| {
            resolve_pinned_selection(snapshot, timestamp, selection, &state)
        })
        .map_err(|error| {
            AppError::from_err(
                ErrorKind::Conflict,
                anyhow::anyhow!("stale selection snapshot {timestamp}: {error}"),
            )
        })??;
    let resolved = ResolvedSelection {
        len: targets.len(),
        targets,
        identity_epoch: state.identity_epoch(),
        selection_epoch: state.selection_epoch(),
    };
    crate::perf_timing!(
        "selection.resolve",
        started,
        "Resolve and validate {} selection targets",
        resolved.len
    );
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    use arrayvec::ArrayString;

    use crate::public::db::tree::state::{SlotRef, TargetSet, TargetSetBuilder, TreeState};
    use crate::public::db::tree_snapshot::read_tree_snapshot::PinnedSnapshotView;
    use crate::public::db::tree_snapshot::{PendingTreeSnapshot, SnapshotBlobView};
    use crate::public::structure::album::Album;

    use super::{
        IndexMembership, NormalizedSelection, SelectionDescriptor, normalize_selection,
        resolve_pinned_selection, resolved_selection_is_current,
    };

    #[test]
    fn selection_wire_format_is_camel_case() {
        let value = serde_json::from_str::<SelectionDescriptor>(
            r#"{"mode":"allExcept","excludedIndices":[1,3]}"#,
        )
        .unwrap();
        assert_eq!(
            value,
            SelectionDescriptor::AllExcept {
                excluded_indices: vec![1, 3]
            }
        );
    }

    #[test]
    fn explicit_and_all_except_values_are_sorted_deduplicated_and_bounded() {
        assert_eq!(
            normalize_selection(
                SelectionDescriptor::Explicit {
                    indices: vec![4, 1, 4, 2],
                },
                5,
            )
            .unwrap(),
            NormalizedSelection::Explicit(vec![1, 2, 4])
        );
        assert_eq!(
            normalize_selection(
                SelectionDescriptor::AllExcept {
                    excluded_indices: vec![3, 1, 3],
                },
                5,
            )
            .unwrap(),
            NormalizedSelection::AllExcept(vec![1, 3])
        );
        assert!(
            normalize_selection(SelectionDescriptor::Explicit { indices: vec![5] }, 5,).is_err()
        );
    }

    #[test]
    fn full_explicit_and_empty_exclusion_use_snapshot_target_set_fast_paths() {
        assert_eq!(
            normalize_selection(
                SelectionDescriptor::Explicit {
                    indices: vec![2, 0, 1, 1],
                },
                3,
            )
            .unwrap(),
            NormalizedSelection::ExplicitAll
        );
        assert_eq!(
            normalize_selection(
                SelectionDescriptor::AllExcept {
                    excluded_indices: Vec::new(),
                },
                3,
            )
            .unwrap(),
            NormalizedSelection::AllExceptNone
        );
        assert_eq!(
            normalize_selection(
                SelectionDescriptor::Explicit {
                    indices: Vec::new(),
                },
                0,
            )
            .unwrap(),
            NormalizedSelection::ExplicitAll
        );
    }

    #[test]
    fn million_item_selection_stays_within_the_working_memory_gate() {
        const ITEM_COUNT: u32 = 1_000_000;
        const MEMORY_GATE_BYTES: usize = 4 * 1024 * 1024 + 256 * 1024;

        let indices = (0..ITEM_COUNT).rev().collect::<Vec<_>>();
        let membership = IndexMembership::with_universe(ITEM_COUNT as usize);
        let working_bytes =
            indices.capacity() * std::mem::size_of::<u32>() + membership.estimated_bytes();
        assert!(working_bytes <= MEMORY_GATE_BYTES);
        drop(membership);

        let normalized = normalize_selection(
            SelectionDescriptor::Explicit { indices },
            ITEM_COUNT as usize,
        )
        .unwrap();
        assert_eq!(normalized, NormalizedSelection::ExplicitAll);
    }

    fn album(id: &str) -> crate::public::structure::abstract_data::AbstractData {
        Album::new(ArrayString::from(id).unwrap(), None).into_abstract_data()
    }

    fn sample_state() -> TreeState {
        TreeState::from_records([
            album("selection-a"),
            album("selection-b"),
            album("selection-c"),
        ])
    }

    fn snapshot_for_state(state: &TreeState) -> PendingTreeSnapshot {
        let slots = state.order.iter().copied().collect::<Vec<_>>();
        PendingTreeSnapshot {
            structural_epoch: state.structural_epoch(),
            identity_epoch: state.identity_epoch(),
            selection_epoch: state.selection_epoch(),
            universe: state.arena.capacity(),
            ordinals: slots.iter().map(|slot_ref| slot_ref.index()).collect(),
            targets: TargetSet::from_unique_slot_refs(slots, state.arena.capacity()),
            scrollbar: Vec::new(),
        }
    }

    fn assert_selection_semantics(snapshot: PinnedSnapshotView<'_>, state: &TreeState) {
        let mut expected = vec![snapshot.slot_ref(0).unwrap(), snapshot.slot_ref(2).unwrap()];
        expected.sort_unstable();
        let targets = resolve_pinned_selection(
            snapshot,
            123,
            SelectionDescriptor::Explicit {
                indices: vec![2, 0, 2],
            },
            state,
        )
        .unwrap();
        assert_eq!(targets.iter().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn memory_and_disk_blob_views_resolve_identical_selections() {
        let state = sample_state();
        let snapshot = snapshot_for_state(&state);
        assert_selection_semantics(PinnedSnapshotView::Memory(&snapshot), &state);

        let bytes = snapshot.encode().unwrap();
        let view = SnapshotBlobView::new(&bytes).unwrap();
        assert_selection_semantics(PinnedSnapshotView::Redb(view), &state);
    }

    #[test]
    fn full_and_excluded_selection_semantics_are_preserved() {
        let state = sample_state();
        let snapshot = snapshot_for_state(&state);
        let all = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::Explicit {
                indices: vec![2, 1, 0],
            },
            &state,
        )
        .unwrap();
        assert_eq!(all, snapshot.targets);

        let all_except_none = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::AllExcept {
                excluded_indices: Vec::new(),
            },
            &state,
        )
        .unwrap();
        assert_eq!(all_except_none, snapshot.targets);

        let one = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::Explicit { indices: vec![1] },
            &state,
        )
        .unwrap();
        assert_eq!(
            one.iter().collect::<Vec<_>>(),
            vec![
                snapshot
                    .targets
                    .slot_ref_for_ordinal(snapshot.ordinals[1])
                    .unwrap()
            ]
        );

        let excluded_override = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::AllExcept {
                excluded_indices: vec![1],
            },
            &state,
        )
        .unwrap();
        let mut expected = vec![
            snapshot
                .targets
                .slot_ref_for_ordinal(snapshot.ordinals[0])
                .unwrap(),
            snapshot
                .targets
                .slot_ref_for_ordinal(snapshot.ordinals[2])
                .unwrap(),
        ];
        expected.sort_unstable();
        assert_eq!(excluded_override.iter().collect::<Vec<_>>(), expected);

        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![3] },
                &state,
            )
            .is_err()
        );
    }

    fn assert_append_compatible(snapshot: PinnedSnapshotView<'_>, state: &TreeState) {
        let expected = snapshot.target_set().unwrap();
        let resolved = resolve_pinned_selection(
            snapshot,
            123,
            SelectionDescriptor::Explicit {
                indices: vec![0, 1, 2],
            },
            state,
        )
        .unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn append_only_changes_keep_memory_and_disk_snapshots_resolvable() {
        let mut state = sample_state();
        let snapshot = snapshot_for_state(&state);
        let bytes = snapshot.encode().unwrap();
        let structural_epoch = state.structural_epoch();
        let identity_epoch = state.identity_epoch();
        let selection_epoch = state.selection_epoch();

        state.insert(&album("selection-new"));

        assert_ne!(state.structural_epoch(), structural_epoch);
        assert_eq!(state.identity_epoch(), identity_epoch);
        assert_eq!(state.selection_epoch(), selection_epoch);
        assert_append_compatible(PinnedSnapshotView::Memory(&snapshot), &state);
        assert_append_compatible(
            PinnedSnapshotView::Redb(SnapshotBlobView::new(&bytes).unwrap()),
            &state,
        );
        assert_eq!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&snapshot),
                123,
                SelectionDescriptor::AllExcept {
                    excluded_indices: Vec::new(),
                },
                &state,
            )
            .unwrap(),
            snapshot.targets,
        );
    }

    #[test]
    fn deletion_validation_only_rejects_selected_snapshot_items() {
        let mut explicit_state = sample_state();
        let explicit_snapshot = snapshot_for_state(&explicit_state);
        let deleted = explicit_snapshot
            .targets
            .slot_ref_for_ordinal(explicit_snapshot.ordinals[0])
            .unwrap();
        explicit_state.remove(deleted).unwrap();
        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&explicit_snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![1] },
                &explicit_state,
            )
            .is_ok()
        );
        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&explicit_snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![0] },
                &explicit_state,
            )
            .is_err()
        );

        let mut excluded_state = sample_state();
        let excluded_snapshot = snapshot_for_state(&excluded_state);
        let deleted = excluded_snapshot
            .targets
            .slot_ref_for_ordinal(excluded_snapshot.ordinals[0])
            .unwrap();
        excluded_state.remove(deleted).unwrap();
        let resolved = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&excluded_snapshot),
            123,
            SelectionDescriptor::AllExcept {
                excluded_indices: vec![0],
            },
            &excluded_state,
        )
        .unwrap();
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn slot_reuse_and_tree_rebuild_never_retarget_old_selections() {
        let mut state = sample_state();
        let snapshot = snapshot_for_state(&state);
        let selected = snapshot
            .targets
            .slot_ref_for_ordinal(snapshot.ordinals[0])
            .unwrap();
        state.remove(selected).unwrap();
        let replacement = state.insert(&album("selection-replacement"));
        assert_eq!(replacement.index(), selected.index());
        assert_ne!(replacement.generation(), selected.generation());
        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![0] },
                &state,
            )
            .is_err()
        );

        let rebuilt = sample_state();
        assert_ne!(rebuilt.identity_epoch(), snapshot.identity_epoch);
        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![0] },
                &rebuilt,
            )
            .is_err()
        );
    }

    #[test]
    fn publication_validation_allows_additions_and_unselected_deletions() {
        let mut state = sample_state();
        let snapshot = snapshot_for_state(&state);
        let selected = TargetSet::from_slot_refs(
            [snapshot
                .targets
                .slot_ref_for_ordinal(snapshot.ordinals[0])
                .unwrap()],
            state.arena.capacity(),
        );
        let identity_epoch = state.identity_epoch();
        let selection_epoch = state.selection_epoch();

        state.insert(&album("selection-new"));
        assert!(resolved_selection_is_current(
            &state,
            identity_epoch,
            selection_epoch,
            &selected,
        ));

        let unselected = snapshot
            .targets
            .slot_ref_for_ordinal(snapshot.ordinals[1])
            .unwrap();
        state.remove(unselected).unwrap();
        assert!(resolved_selection_is_current(
            &state,
            identity_epoch,
            selection_epoch,
            &selected,
        ));

        state.remove(selected.iter().next().unwrap()).unwrap();
        assert!(!resolved_selection_is_current(
            &state,
            identity_epoch,
            selection_epoch,
            &selected,
        ));
        assert!(!resolved_selection_is_current(
            &sample_state(),
            identity_epoch,
            selection_epoch,
            &selected,
        ));
    }

    fn median(mut samples: Vec<Duration>) -> Duration {
        samples.sort_unstable();
        samples[samples.len() / 2]
    }

    fn legacy_explicit_all(mut indices: Vec<u32>, universe: usize) -> TargetSet {
        indices.sort_unstable();
        indices.dedup();
        let mut targets = TargetSetBuilder::default();
        for ordinal in indices {
            targets.insert(SlotRef::new(ordinal, 1));
        }
        targets.finish(universe)
    }

    fn optimized_explicit_all(
        indices: Vec<u32>,
        snapshot_targets: &TargetSet,
        universe: usize,
    ) -> TargetSet {
        match normalize_selection(SelectionDescriptor::Explicit { indices }, universe).unwrap() {
            NormalizedSelection::ExplicitAll => snapshot_targets.clone(),
            NormalizedSelection::Explicit(indices) => {
                let mut targets = TargetSetBuilder::default();
                for ordinal in indices {
                    targets.insert(SlotRef::new(ordinal, 1));
                }
                targets.finish(universe)
            }
            _ => unreachable!("explicit selection changed mode"),
        }
    }

    #[test]
    #[ignore = "local 1M explicit-all selection microbenchmark"]
    fn benchmark_million_item_explicit_all_fast_path() {
        const ITEM_COUNT: u32 = 1_000_000;
        let input = (0..ITEM_COUNT).rev().collect::<Vec<_>>();
        let mut words = vec![u64::MAX; (ITEM_COUNT as usize).div_ceil(64)];
        let trailing_bits = ITEM_COUNT as usize % 64;
        if trailing_bits != 0 {
            *words.last_mut().unwrap() = (1_u64 << trailing_bits) - 1;
        }
        let snapshot_targets = TargetSet::from_dense_parts(words, Vec::new());

        for _ in 0..2 {
            black_box(legacy_explicit_all(input.clone(), ITEM_COUNT as usize));
            black_box(optimized_explicit_all(
                input.clone(),
                &snapshot_targets,
                ITEM_COUNT as usize,
            ));
        }

        let mut legacy_samples = Vec::with_capacity(9);
        let mut optimized_samples = Vec::with_capacity(9);
        for _ in 0..9 {
            let started = Instant::now();
            black_box(legacy_explicit_all(input.clone(), ITEM_COUNT as usize));
            legacy_samples.push(started.elapsed());

            let started = Instant::now();
            black_box(optimized_explicit_all(
                input.clone(),
                &snapshot_targets,
                ITEM_COUNT as usize,
            ));
            optimized_samples.push(started.elapsed());
        }
        let legacy = median(legacy_samples);
        let optimized = median(optimized_samples);
        eprintln!(
            "selection explicit-all legacy={legacy:?} optimized={optimized:?} speedup={:.2}x",
            legacy.as_secs_f64() / optimized.as_secs_f64()
        );
        assert!(optimized < legacy);
    }
}
