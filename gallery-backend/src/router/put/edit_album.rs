use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use arrayvec::ArrayString;
use rocket::serde::{Deserialize, json::Json};
use serde::Serialize;

use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::object::next_mutation_timestamp;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::selection::{SelectionDescriptor, resolve_selection};
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditAlbumsData {
    #[serde(default)]
    index_array: Vec<u32>,
    #[serde(default)]
    selection: Option<SelectionDescriptor>,
    add_albums_array: Vec<ArrayString<64>>,
    remove_albums_array: Vec<ArrayString<64>>,
    timestamp: i64,
}

#[put("/put/edit_album", format = "json", data = "<json_data>")]
pub async fn edit_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<EditAlbumsData>,
) -> AppResult<()> {
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
    let mut add = data.add_albums_array.into_iter().collect::<BTreeSet<_>>();
    let remove = data
        .remove_albums_array
        .into_iter()
        .collect::<BTreeSet<_>>();
    add.retain(|album| !remove.contains(album));
    let structural_epoch = resolved.structural_epoch;
    let targets = resolved.targets;
    let reservation = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        if state.structural_epoch() != structural_epoch {
            return Err(AppError::new(ErrorKind::Conflict, "selection is stale"));
        }
        let membership_bytes = add
            .iter()
            .map(|album_id| DirtyOperation::Albums {
                targets: targets.clone(),
                add: BTreeSet::from([*album_id]),
                remove: BTreeSet::new(),
            })
            .chain(remove.iter().map(|album_id| DirtyOperation::Albums {
                targets: targets.clone(),
                add: BTreeSet::new(),
                remove: BTreeSet::from([*album_id]),
            }))
            .map(|operation| operation.estimated_bytes())
            .sum::<usize>();
        let universe = state.arena.capacity();
        let mut touch_targets = targets.clone();
        let album_targets = crate::public::db::tree::state::TargetSet::from_slot_refs(
            add.union(&remove)
                .filter_map(|album_id| state.find(album_id.as_str())),
            universe,
        );
        touch_targets.union(&album_targets, universe);
        let touch_bytes = DirtyOperation::Touch {
            targets: touch_targets,
            changed_at: 0,
        }
        .estimated_bytes();
        membership_bytes
            + add
                .union(&remove)
                .filter_map(|album_id| state.albums.get(album_id))
                .map(|album| DirtyOperation::AlbumReplace(album.clone()).estimated_bytes() + 1_024)
                .sum::<usize>()
            + touch_bytes
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
    if state.structural_epoch() != structural_epoch {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection became stale before publication",
        ));
    }
    let targets = state.media_targets(&targets);
    if let Some(missing) = add
        .iter()
        .find(|album_id| !state.albums.contains_key(*album_id))
    {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::NotFound,
            format!("album {missing} does not exist"),
        ));
    }
    let universe = state.arena.capacity();
    let mut operations = Vec::with_capacity(add.len() + remove.len());
    for album_id in &add {
        let existing = state.query.albums.get(album_id);
        let changed = crate::public::db::tree::state::TargetSet::from_unique_slot_refs(
            targets
                .iter()
                .filter(|slot_ref| !existing.is_some_and(|set| set.contains(slot_ref.index()))),
            universe,
        );
        if !changed.is_empty() {
            operations.push(DirtyOperation::Albums {
                targets: changed,
                add: BTreeSet::from([*album_id]),
                remove: BTreeSet::new(),
            });
        }
    }
    for album_id in &remove {
        let existing = state.query.albums.get(album_id);
        let changed = crate::public::db::tree::state::TargetSet::from_unique_slot_refs(
            targets
                .iter()
                .filter(|slot_ref| existing.is_some_and(|set| set.contains(slot_ref.index()))),
            universe,
        );
        if !changed.is_empty() {
            operations.push(DirtyOperation::Albums {
                targets: changed,
                add: BTreeSet::new(),
                remove: BTreeSet::from([*album_id]),
            });
        }
    }
    let changed_at = next_mutation_timestamp();
    let album_patches = state.edit_album_memberships(targets.ordinals(), &add, &remove, changed_at);
    let mut touch_targets = targets.clone();
    let album_targets = crate::public::db::tree::state::TargetSet::from_slot_refs(
        album_patches
            .iter()
            .filter_map(|album| state.find(album.object.id.as_str())),
        universe,
    );
    touch_targets.union(&album_targets, universe);
    operations.extend(album_patches.into_iter().map(DirtyOperation::AlbumReplace));
    if !touch_targets.is_empty() {
        operations.push(DirtyOperation::Touch {
            targets: touch_targets,
            changed_at,
        });
    }
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
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumCover {
    pub album_id: ArrayString<64>,
    pub cover_hash: ArrayString<64>,
}

#[put("/put/set_album_cover", data = "<set_album_cover>")]
pub async fn set_album_cover(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    set_album_cover: Json<SetAlbumCover>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = set_album_cover.into_inner();
    let reservation = TREE
        .state
        .read()
        .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?
        .albums
        .get(&data.album_id)
        .map(|album| DirtyOperation::AlbumReplace(album.clone()).estimated_bytes() + 1_024)
        .ok_or_else(|| AppError::new(ErrorKind::NotFound, "album not found"))?;
    WRITE_BEHIND.reserve(reservation).await?;
    let mut state = TREE.state.write().map_err(|_| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::Internal, "tree state lock poisoned")
    })?;
    let cover = state
        .find(data.cover_hash.as_str())
        .and_then(|slot_ref| state.get(slot_ref))
        .cloned()
        .ok_or_else(|| {
            WRITE_BEHIND.release_reservation(reservation);
            AppError::new(ErrorKind::NotFound, "cover media not found")
        })?;
    if !state
        .query
        .albums
        .get(&data.album_id)
        .is_some_and(|members| {
            members.contains(state.find(data.cover_hash.as_str()).unwrap().index())
        })
    {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "cover must be a member of the album",
        ));
    }
    let universe = state.arena.capacity();
    let album_slot = state.find(data.album_id.as_str()).ok_or_else(|| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::NotFound, "album not found")
    })?;
    let changed_at = next_mutation_timestamp();
    let album = state.albums.get_mut(&data.album_id).ok_or_else(|| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::NotFound, "album not found")
    })?;
    album.metadata.cover = Some(cover.id);
    album.object.thumbhash = cover.thumbhash_vec();
    album.object.cache_version = cover.cache_version;
    album.object.touch_update_at(changed_at);
    let album = album.clone();
    WRITE_BEHIND.enqueue_reserved(DirtyOperation::AlbumReplace(album), reservation);
    WRITE_BEHIND.enqueue_reserved(
        DirtyOperation::Touch {
            targets: crate::public::db::tree::state::TargetSet::from_slot_refs(
                [album_slot],
                universe,
            ),
            changed_at,
        },
        0,
    );
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumTitle {
    pub album_id: ArrayString<64>,
    pub title: Option<String>,
}

#[put("/put/set_album_title", data = "<set_album_title>")]
pub async fn set_album_title(
    auth: GuardResult<GuardShare>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    set_album_title: Json<SetAlbumTitle>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = set_album_title.into_inner();
    let reservation = TREE
        .state
        .read()
        .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?
        .albums
        .get(&data.album_id)
        .map(|album| {
            DirtyOperation::AlbumReplace(album.clone()).estimated_bytes()
                + data.title.as_ref().map_or(0, String::capacity)
                + 1_024
        })
        .ok_or_else(|| AppError::new(ErrorKind::NotFound, "album not found"))?;
    WRITE_BEHIND.reserve(reservation).await?;
    let mut state = TREE.state.write().map_err(|_| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::Internal, "tree state lock poisoned")
    })?;
    let universe = state.arena.capacity();
    let album_slot = state.find(data.album_id.as_str()).ok_or_else(|| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::NotFound, "album not found")
    })?;
    let changed_at = next_mutation_timestamp();
    let album = state.albums.get_mut(&data.album_id).ok_or_else(|| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::NotFound, "album not found")
    })?;
    album.metadata.title = data.title;
    album.object.touch_update_at(changed_at);
    let album = album.clone();
    WRITE_BEHIND.enqueue_reserved(DirtyOperation::AlbumReplace(album), reservation);
    WRITE_BEHIND.enqueue_reserved(
        DirtyOperation::Touch {
            targets: crate::public::db::tree::state::TargetSet::from_slot_refs(
                [album_slot],
                universe,
            ),
            changed_at,
        },
        0,
    );
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    Ok(())
}
