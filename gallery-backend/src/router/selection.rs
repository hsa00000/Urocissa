use rocket::serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::operations::open_db::open_tree_snapshot_table;
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::{TargetSet, TargetSetBuilder};
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
    pub structural_epoch: u64,
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
    universe: usize,
    timestamp: i64,
) -> Result<TargetSet, AppError> {
    let mut targets = TargetSetBuilder::default();
    snapshot
        .for_each_selected_slot_ref(indices, |slot_ref| {
            targets.insert(slot_ref);
        })
        .map_err(|error| {
            AppError::from_err(
                ErrorKind::Conflict,
                anyhow::anyhow!("stale selection snapshot {timestamp}: {error}"),
            )
        })?;
    Ok(targets.finish(universe))
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
    structural_epoch: u64,
    universe: usize,
) -> Result<TargetSet, AppError> {
    let normalized = normalize_selection(selection, snapshot.len())?;
    if snapshot.structural_epoch() != structural_epoch || snapshot.universe() != universe {
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection snapshot belongs to an older structural epoch",
        ));
    }

    match normalized {
        NormalizedSelection::ExplicitAll | NormalizedSelection::AllExceptNone => {
            snapshot_target_set(&snapshot, timestamp)
        }
        NormalizedSelection::Explicit(indices) => {
            resolve_indices(&snapshot, &indices, universe, timestamp)
        }
        NormalizedSelection::AllExcept(excluded) => {
            let mut targets = snapshot_target_set(&snapshot, timestamp)?;
            let excluded_targets = resolve_indices(&snapshot, &excluded, universe, timestamp)?;
            targets.subtract(&excluded_targets);
            Ok(targets)
        }
    }
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

    let (structural_epoch, universe) = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        (state.structural_epoch(), state.arena.capacity())
    };
    let targets = snapshot
        .with_pinned_view(|snapshot| {
            resolve_pinned_selection(snapshot, timestamp, selection, structural_epoch, universe)
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
        structural_epoch,
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

    use crate::public::db::tree::state::{SlotRef, TargetSet, TargetSetBuilder};
    use crate::public::db::tree_snapshot::read_tree_snapshot::PinnedSnapshotView;
    use crate::public::db::tree_snapshot::{PendingTreeSnapshot, SnapshotBlobView};

    use super::{
        IndexMembership, NormalizedSelection, SelectionDescriptor, normalize_selection,
        resolve_pinned_selection,
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

    fn sample_snapshot() -> PendingTreeSnapshot {
        let slots = [SlotRef::new(4, 1), SlotRef::new(2, 3), SlotRef::new(7, 1)];
        PendingTreeSnapshot {
            structural_epoch: 42,
            universe: 8,
            ordinals: slots.iter().map(|slot_ref| slot_ref.index()).collect(),
            targets: TargetSet::from_unique_slot_refs(slots, 8),
            scrollbar: Vec::new(),
        }
    }

    fn assert_selection_semantics(snapshot: PinnedSnapshotView<'_>) {
        let targets = resolve_pinned_selection(
            snapshot,
            123,
            SelectionDescriptor::Explicit {
                indices: vec![2, 0, 2],
            },
            42,
            8,
        )
        .unwrap();
        assert_eq!(
            targets.iter().collect::<Vec<_>>(),
            vec![SlotRef::new(4, 1), SlotRef::new(7, 1)]
        );
    }

    #[test]
    fn memory_and_disk_blob_views_resolve_identical_selections() {
        let snapshot = sample_snapshot();
        assert_selection_semantics(PinnedSnapshotView::Memory(&snapshot));

        let bytes = snapshot.encode().unwrap();
        let view = SnapshotBlobView::new(&bytes).unwrap();
        assert_selection_semantics(PinnedSnapshotView::Redb(view));
    }

    #[test]
    fn full_exclusion_generation_and_stale_snapshot_validation_are_preserved() {
        let snapshot = sample_snapshot();
        let all = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::Explicit {
                indices: vec![2, 1, 0],
            },
            42,
            8,
        )
        .unwrap();
        assert_eq!(all, snapshot.targets);

        let all_except_none = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::AllExcept {
                excluded_indices: Vec::new(),
            },
            42,
            8,
        )
        .unwrap();
        assert_eq!(all_except_none, snapshot.targets);

        let generation_override = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::Explicit { indices: vec![1] },
            42,
            8,
        )
        .unwrap();
        assert_eq!(
            generation_override.iter().collect::<Vec<_>>(),
            vec![SlotRef::new(2, 3)]
        );

        let excluded_override = resolve_pinned_selection(
            PinnedSnapshotView::Memory(&snapshot),
            123,
            SelectionDescriptor::AllExcept {
                excluded_indices: vec![1],
            },
            42,
            8,
        )
        .unwrap();
        assert_eq!(
            excluded_override.iter().collect::<Vec<_>>(),
            vec![SlotRef::new(4, 1), SlotRef::new(7, 1)]
        );

        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![3] },
                42,
                8,
            )
            .is_err()
        );
        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![0] },
                43,
                8,
            )
            .is_err()
        );
        assert!(
            resolve_pinned_selection(
                PinnedSnapshotView::Memory(&snapshot),
                123,
                SelectionDescriptor::Explicit { indices: vec![0] },
                42,
                9,
            )
            .is_err()
        );
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
