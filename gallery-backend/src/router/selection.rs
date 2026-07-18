use rocket::serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::operations::open_db::open_tree_snapshot_table;
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::{SlotRef, TargetSet};
use crate::public::error::{AppError, ErrorKind};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SelectionDescriptor {
    Explicit { indices: Vec<usize> },
    AllExcept { excluded_indices: Vec<usize> },
}

impl SelectionDescriptor {
    pub fn explicit(indices: Vec<usize>) -> Self {
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
    Explicit(Vec<usize>),
    AllExcept(Vec<usize>),
}

fn normalize_selection(
    selection: &SelectionDescriptor,
    snapshot_len: usize,
) -> Result<NormalizedSelection, AppError> {
    let (values, explicit) = match selection {
        SelectionDescriptor::Explicit { indices } => (indices, true),
        SelectionDescriptor::AllExcept { excluded_indices } => (excluded_indices, false),
    };
    let mut values = values.clone();
    values.sort_unstable();
    values.dedup();
    if values.last().is_some_and(|index| *index >= snapshot_len) {
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection contains an out-of-range index",
        ));
    }
    Ok(if explicit {
        NormalizedSelection::Explicit(values)
    } else {
        NormalizedSelection::AllExcept(values)
    })
}

/// Resolve a UI selection against its immutable tree snapshot and validate the
/// complete generational identity set before any mutation is published.
pub fn resolve_selection(
    timestamp: i64,
    selection: &SelectionDescriptor,
) -> Result<ResolvedSelection, AppError> {
    let started = Instant::now();
    let snapshot = open_tree_snapshot_table(timestamp).map_err(|error| {
        AppError::from_err(
            ErrorKind::Conflict,
            anyhow::anyhow!("stale selection snapshot {timestamp}: {error}"),
        )
    })?;
    let snapshot_len = snapshot.len();
    let normalized = normalize_selection(selection, snapshot_len)?;

    let (structural_epoch, universe) = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        (state.structural_epoch(), state.arena.capacity())
    };
    let snapshot_epoch = snapshot.structural_epoch().map_err(|error| {
        AppError::from_err(
            ErrorKind::Conflict,
            anyhow::anyhow!("stale selection snapshot {timestamp}: {error}"),
        )
    })?;
    if snapshot_epoch != structural_epoch || snapshot.universe().unwrap_or(usize::MAX) != universe {
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection snapshot belongs to an older structural epoch",
        ));
    }

    let targets = match &normalized {
        NormalizedSelection::Explicit(indices) => {
            let slots = indices
                .iter()
                .copied()
                .map(|index| {
                    snapshot
                        .get_slot_ref(index)
                        .map(SlotRef::from_raw)
                        .map_err(|error| {
                            AppError::from_err(
                                ErrorKind::Conflict,
                                anyhow::anyhow!("stale selection index {index}: {error}"),
                            )
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            TargetSet::from_unique_slot_refs(slots, universe)
        }
        NormalizedSelection::AllExcept(excluded) => {
            let mut targets = snapshot.target_set().map_err(|error| {
                AppError::from_err(
                    ErrorKind::Conflict,
                    anyhow::anyhow!("invalid selection target bitmap: {error}"),
                )
            })?;
            let excluded_targets = TargetSet::from_unique_slot_refs(
                excluded
                    .iter()
                    .copied()
                    .map(|index| snapshot.get_slot_ref(index).map(SlotRef::from_raw))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                universe,
            );
            targets.subtract(&excluded_targets);
            targets
        }
    };
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
    use super::{NormalizedSelection, SelectionDescriptor, normalize_selection};

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
                &SelectionDescriptor::Explicit {
                    indices: vec![4, 1, 4, 2],
                },
                5,
            )
            .unwrap(),
            NormalizedSelection::Explicit(vec![1, 2, 4])
        );
        assert_eq!(
            normalize_selection(
                &SelectionDescriptor::AllExcept {
                    excluded_indices: vec![3, 1, 3],
                },
                5,
            )
            .unwrap(),
            NormalizedSelection::AllExcept(vec![1, 3])
        );
        assert!(
            normalize_selection(&SelectionDescriptor::Explicit { indices: vec![5] }, 5,).is_err()
        );
    }
}
