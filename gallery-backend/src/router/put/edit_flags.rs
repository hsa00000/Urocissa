use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use rocket::serde::{Deserialize, json::Json};

use crate::public::db::tree::state::FlagPatch;
use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::selection::{SelectionDescriptor, resolve_selection};
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditFlagsData {
    #[serde(default)]
    index_array: Vec<usize>,
    #[serde(default)]
    selection: Option<SelectionDescriptor>,
    timestamp: i64,
    #[serde(default)]
    is_favorite: Option<bool>,
    #[serde(default)]
    is_archived: Option<bool>,
    #[serde(default)]
    is_trashed: Option<bool>,
}

#[put("/put/edit_flags", format = "json", data = "<json_data>")]
pub async fn edit_flags(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<EditFlagsData>,
) -> AppResult<Json<()>> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = json_data.into_inner();
    let selection = data
        .selection
        .unwrap_or_else(|| SelectionDescriptor::explicit(data.index_array));
    let resolved =
        tokio::task::spawn_blocking(move || resolve_selection(data.timestamp, &selection))
            .await
            .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;
    let patch = FlagPatch {
        favorite: data.is_favorite,
        archived: data.is_archived,
        trashed: data.is_trashed,
    };
    let targets = resolved.targets;
    let (affected_albums, reservation) = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        if !targets.is_current(&state) {
            return Err(AppError::new(ErrorKind::Conflict, "selection is stale"));
        }
        let affected = if patch.trashed.is_some() {
            state
                .query
                .albums
                .iter()
                .filter(|(_, members)| {
                    members
                        .iter()
                        .any(|ordinal| targets.ordinals().contains(ordinal))
                })
                .map(|(album_id, _)| *album_id)
                .collect::<BTreeSet<_>>()
        } else {
            BTreeSet::new()
        };
        let album_bytes = affected
            .iter()
            .filter_map(|album_id| state.albums.get(album_id))
            .map(|album| DirtyOperation::AlbumReplace(album.clone()).estimated_bytes() + 1_024)
            .sum::<usize>();
        let flag_bytes = [
            patch.favorite.map(|value| DirtyOperation::Flags {
                targets: targets.clone(),
                favorite: Some(value),
                archived: None,
                trashed: None,
            }),
            patch.archived.map(|value| DirtyOperation::Flags {
                targets: targets.clone(),
                favorite: None,
                archived: Some(value),
                trashed: None,
            }),
            patch.trashed.map(|value| DirtyOperation::Flags {
                targets: targets.clone(),
                favorite: None,
                archived: None,
                trashed: Some(value),
            }),
        ]
        .into_iter()
        .flatten()
        .map(|operation| operation.estimated_bytes())
        .sum::<usize>();
        (affected, flag_bytes + album_bytes)
    };
    WRITE_BEHIND.reserve(reservation).await?;
    let mut state = match TREE.state.write() {
        Ok(state) => state,
        Err(_) => {
            WRITE_BEHIND.release_reservation(reservation);
            return Err(AppError::new(
                ErrorKind::Internal,
                "tree state lock poisoned",
            ));
        }
    };
    if !targets.is_current(&state) {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection became stale before publication",
        ));
    }
    let current_affected = if patch.trashed.is_some() {
        state
            .query
            .albums
            .iter()
            .filter(|(_, members)| {
                members
                    .iter()
                    .any(|ordinal| targets.ordinals().contains(ordinal))
            })
            .map(|(album_id, _)| *album_id)
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    if current_affected != affected_albums {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "album memberships changed before flag publication",
        ));
    }

    let universe = state.arena.capacity();
    let mut operations = Vec::with_capacity(3);
    if let Some(value) = patch.favorite {
        let changed = crate::public::db::tree::state::TargetSet::from_slot_refs(
            targets
                .iter()
                .filter(|slot_ref| state.query.favorite.contains(slot_ref.index()) != value),
            universe,
        );
        if !changed.is_empty() {
            operations.push(DirtyOperation::Flags {
                targets: changed,
                favorite: Some(value),
                archived: None,
                trashed: None,
            });
        }
    }
    if let Some(value) = patch.archived {
        let changed = crate::public::db::tree::state::TargetSet::from_slot_refs(
            targets
                .iter()
                .filter(|slot_ref| state.query.archived.contains(slot_ref.index()) != value),
            universe,
        );
        if !changed.is_empty() {
            operations.push(DirtyOperation::Flags {
                targets: changed,
                favorite: None,
                archived: Some(value),
                trashed: None,
            });
        }
    }
    let (trash_changed, trash_value) = if let Some(value) = patch.trashed {
        let changed = crate::public::db::tree::state::TargetSet::from_slot_refs(
            targets
                .iter()
                .filter(|slot_ref| state.query.trashed.contains(slot_ref.index()) != value),
            universe,
        );
        if !changed.is_empty() {
            operations.push(DirtyOperation::Flags {
                targets: changed.clone(),
                favorite: None,
                archived: None,
                trashed: Some(value),
            });
        }
        (Some(changed), Some(value))
    } else {
        (None, None)
    };

    state.query.edit_flags(
        targets.ordinals(),
        FlagPatch {
            favorite: patch.favorite,
            archived: patch.archived,
            trashed: None,
        },
    );
    let album_patches = match (trash_changed, trash_value) {
        (Some(changed), Some(value)) if !changed.is_empty() => state.edit_flags_and_refresh(
            changed.ordinals(),
            FlagPatch {
                favorite: None,
                archived: None,
                trashed: Some(value),
            },
        ),
        _ => Vec::new(),
    };
    operations.extend(album_patches.into_iter().map(DirtyOperation::AlbumReplace));
    if operations.is_empty() {
        WRITE_BEHIND.release_reservation(reservation);
    } else {
        let mut reservation_left = reservation;
        for operation in operations {
            WRITE_BEHIND.enqueue_reserved(operation, reservation_left);
            reservation_left = 0;
        }
    }
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    Ok(Json(()))
}
