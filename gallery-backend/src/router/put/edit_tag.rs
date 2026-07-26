use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use rocket::serde::{Deserialize, json::Json};

use crate::public::db::tree::read_tags::FacetValueInfo;
use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::object::next_mutation_timestamp;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::selection::{SelectionDescriptor, resolve_selection};
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditTagsData {
    #[serde(default)]
    index_array: Vec<u32>,
    #[serde(default)]
    selection: Option<SelectionDescriptor>,
    add_tags_array: Vec<String>,
    remove_tags_array: Vec<String>,
    timestamp: i64,
}

#[put("/put/edit_tag", format = "json", data = "<json_data>")]
pub async fn edit_tag(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<EditTagsData>,
) -> AppResult<Json<Vec<FacetValueInfo>>> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = json_data.into_inner();
    let selection = data
        .selection
        .unwrap_or_else(|| SelectionDescriptor::explicit(data.index_array));
    let resolved =
        tokio::task::spawn_blocking(move || resolve_selection(data.timestamp, selection))
            .await
            .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;

    let mut add = data.add_tags_array.into_iter().collect::<BTreeSet<_>>();
    let remove = data.remove_tags_array.into_iter().collect::<BTreeSet<_>>();
    add.retain(|tag| !remove.contains(tag));
    let structural_epoch = resolved.structural_epoch;
    let targets = resolved.targets;
    let worst_case_operations = add
        .iter()
        .map(|tag| DirtyOperation::Tags {
            targets: targets.clone(),
            add: BTreeSet::from([tag.clone()]),
            remove: BTreeSet::new(),
        })
        .chain(remove.iter().map(|tag| DirtyOperation::Tags {
            targets: targets.clone(),
            add: BTreeSet::new(),
            remove: BTreeSet::from([tag.clone()]),
        }))
        .collect::<Vec<_>>();
    let touch_preview = DirtyOperation::Touch {
        targets: targets.clone(),
        changed_at: 0,
    };
    let bytes = worst_case_operations
        .iter()
        .map(DirtyOperation::estimated_bytes)
        .sum::<usize>()
        + touch_preview.estimated_bytes();
    WRITE_BEHIND.reserve(bytes).await?;

    let mut state = match TREE.state.write() {
        Ok(state) => state,
        Err(_) => {
            WRITE_BEHIND.release_reservation(bytes);
            return Err(AppError::new(
                ErrorKind::Internal,
                "tree state lock poisoned",
            ));
        }
    };
    if state.structural_epoch() != structural_epoch {
        WRITE_BEHIND.release_reservation(bytes);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection became stale before publication",
        ));
    }
    let universe = state.arena.capacity();
    let mut operations = Vec::with_capacity(add.len() + remove.len());
    for tag in &add {
        let existing = state.query.tags.get(tag);
        let changed = crate::public::db::tree::state::TargetSet::from_unique_slot_refs(
            targets
                .iter()
                .filter(|slot_ref| !existing.is_some_and(|set| set.contains(slot_ref.index()))),
            universe,
        );
        if !changed.is_empty() {
            operations.push(DirtyOperation::Tags {
                targets: changed,
                add: BTreeSet::from([tag.clone()]),
                remove: BTreeSet::new(),
            });
        }
    }
    for tag in &remove {
        let existing = state.query.tags.get(tag);
        let changed = crate::public::db::tree::state::TargetSet::from_unique_slot_refs(
            targets
                .iter()
                .filter(|slot_ref| existing.is_some_and(|set| set.contains(slot_ref.index()))),
            universe,
        );
        if !changed.is_empty() {
            operations.push(DirtyOperation::Tags {
                targets: changed,
                add: BTreeSet::new(),
                remove: BTreeSet::from([tag.clone()]),
            });
        }
    }
    let changed_at = next_mutation_timestamp();
    state
        .query
        .edit_tags(targets.ordinals(), &add, &remove, universe);
    state.edit_cached_album_objects(&targets, changed_at, |object| {
        object.tags.extend(add.iter().cloned());
        for tag in &remove {
            object.tags.remove(tag);
        }
    });
    if !targets.is_empty() {
        operations.push(DirtyOperation::Touch {
            targets,
            changed_at,
        });
    }
    if operations.is_empty() {
        WRITE_BEHIND.release_reservation(bytes);
    } else {
        let mut reservation_left = bytes;
        for operation in operations {
            WRITE_BEHIND.enqueue_reserved(operation, reservation_left);
            reservation_left = 0;
        }
    }
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    drop(state);

    Ok(Json(TREE.read_tags()))
}
