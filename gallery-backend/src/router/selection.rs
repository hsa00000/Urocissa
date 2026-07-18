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

    let state = TREE
        .state
        .read()
        .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
    let universe = state.arena.capacity();
    let mut slots = Vec::new();
    let mut len = 0;
    let mut validate_identity = |index: usize, raw: u64| -> Result<(), AppError> {
        let slot_ref = SlotRef::from_raw(raw);
        state.get(slot_ref).ok_or_else(|| {
            AppError::new(
                ErrorKind::Conflict,
                format!("selection index {index} has a stale generation"),
            )
        })?;
        slots.push(slot_ref);
        len += 1;
        Ok(())
    };
    if let NormalizedSelection::Explicit(indices) = &normalized {
        for index in indices.iter().copied() {
            let raw = snapshot.get_slot_ref(index).map_err(|error| {
                AppError::from_err(
                    ErrorKind::Conflict,
                    anyhow::anyhow!("stale selection index {index}: {error}"),
                )
            })?;
            validate_identity(index, raw)?;
        }
    } else if let NormalizedSelection::AllExcept(excluded) = &normalized {
        let mut excluded_cursor = 0;
        snapshot
            .for_each_slot_ref(|index, raw| {
                if excluded.get(excluded_cursor) == Some(&index) {
                    excluded_cursor += 1;
                    Ok(())
                } else {
                    validate_identity(index, raw).map_err(anyhow::Error::new)
                }
            })
            .map_err(|error| {
                error
                    .downcast::<AppError>()
                    .unwrap_or_else(|error| AppError::from_err(ErrorKind::Conflict, error))
            })?;
    }
    let resolved = ResolvedSelection {
        targets: TargetSet::from_slot_refs(slots, universe),
        len,
    };
    crate::perf_timing!(
        "selection.resolve",
        started,
        "Resolve and validate selection"
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
