use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use rocket::serde::{Deserialize, json::Json};

use crate::public::db::tree::state::TargetSet;
use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::object::ObjectType;
use crate::public::structure::object::next_mutation_timestamp;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::selection::{SelectionDescriptor, resolve_selection};
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteList {
    #[serde(default)]
    delete_list: Vec<usize>,
    #[serde(default)]
    selection: Option<SelectionDescriptor>,
    timestamp: i64,
}

#[delete("/delete/delete-data", format = "json", data = "<json_data>")]
pub async fn delete_data(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<DeleteList>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = json_data.into_inner();
    let selection = data
        .selection
        .unwrap_or_else(|| SelectionDescriptor::explicit(data.delete_list));
    let resolved =
        tokio::task::spawn_blocking(move || resolve_selection(data.timestamp, &selection))
            .await
            .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;

    let contains_media = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        if state.structural_epoch() != resolved.structural_epoch {
            return Err(AppError::new(ErrorKind::Conflict, "selection is stale"));
        }
        resolved.targets.iter().any(|ordinal| {
            state
                .get(ordinal)
                .is_some_and(|record| record.object_type != ObjectType::Album)
        })
    };
    if contains_media {
        delete_durable_selection(resolved.targets, resolved.structural_epoch).await
    } else {
        delete_logical_albums(resolved.targets, resolved.structural_epoch).await
    }
}

async fn delete_logical_albums(targets: TargetSet, structural_epoch: u64) -> AppResult<()> {
    let reservation = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        if state.structural_epoch() != structural_epoch {
            return Err(AppError::new(
                ErrorKind::Conflict,
                "album selection is stale",
            ));
        }
        targets
            .iter()
            .filter_map(|slot_ref| state.get(slot_ref).map(|record| record.id))
            .map(|album_id| {
                let delete_bytes = DirtyOperation::AlbumDelete(album_id).estimated_bytes();
                let membership_bytes = state
                    .query
                    .albums
                    .get(&album_id)
                    .map(|members| DirtyOperation::Albums {
                        targets: TargetSet::from_slot_refs(
                            members
                                .iter()
                                .filter_map(|ordinal| state.slot_for_ordinal(ordinal)),
                            state.arena.capacity(),
                        ),
                        add: BTreeSet::new(),
                        remove: BTreeSet::from([album_id]),
                    })
                    .map_or(0, |operation| {
                        let touch_bytes = match &operation {
                            DirtyOperation::Albums { targets, .. } => DirtyOperation::Touch {
                                targets: targets.clone(),
                                changed_at: 0,
                            }
                            .estimated_bytes(),
                            _ => 0,
                        };
                        operation.estimated_bytes() + touch_bytes
                    });
                delete_bytes + membership_bytes
            })
            .sum::<usize>()
            .max(256)
    };
    WRITE_BEHIND.reserve(reservation).await?;
    let mut state = TREE.state.write().map_err(|_| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::Internal, "tree state lock poisoned")
    })?;
    if state.structural_epoch() != structural_epoch {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "album selection became stale",
        ));
    }
    let album_slots = targets.iter().collect::<Vec<_>>();
    let mut operations = Vec::new();
    let universe = state.arena.capacity();
    let mut touch_targets = TargetSet::default();
    let mut deleted_album_ids = BTreeSet::new();
    for slot_ref in &album_slots {
        let Some(record) = state.get(*slot_ref) else {
            WRITE_BEHIND.release_reservation(reservation);
            return Err(AppError::new(
                ErrorKind::Conflict,
                "album selection became stale",
            ));
        };
        if record.object_type != ObjectType::Album {
            WRITE_BEHIND.release_reservation(reservation);
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "selection contains media",
            ));
        }
        let album_id = record.id;
        deleted_album_ids.insert(album_id);
        let members = state
            .query
            .albums
            .get(&album_id)
            .cloned()
            .unwrap_or_default();
        if !members.is_empty() {
            let member_targets = TargetSet::from_slot_refs(
                members
                    .iter()
                    .filter_map(|ordinal| state.slot_for_ordinal(ordinal)),
                universe,
            );
            touch_targets.union(&member_targets, universe);
            operations.push(DirtyOperation::Albums {
                targets: member_targets,
                add: BTreeSet::new(),
                remove: BTreeSet::from([album_id]),
            });
        }
        operations.push(DirtyOperation::AlbumDelete(album_id));
    }
    if !touch_targets.is_empty() {
        operations.push(DirtyOperation::Touch {
            targets: touch_targets,
            changed_at: next_mutation_timestamp(),
        });
    }
    let actual_bytes = operations
        .iter()
        .map(DirtyOperation::estimated_bytes)
        .sum::<usize>();
    if actual_bytes > reservation {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "album membership changed before delete publication",
        ));
    }
    for operation in &operations {
        if let DirtyOperation::Albums {
            targets, remove, ..
        } = operation
        {
            let universe = state.arena.capacity();
            state
                .query
                .edit_albums(targets.ordinals(), &BTreeSet::new(), remove, universe);
        }
    }
    WRITE_BEHIND.cancel_targets(&targets, &deleted_album_ids);
    state.remove_targets(&targets);
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

async fn delete_durable_selection(targets: TargetSet, structural_epoch: u64) -> AppResult<()> {
    tokio::task::spawn_blocking(move || -> AppResult<()> {
        let _persistence_guard = TREE.persistence_lock.lock().unwrap();
        let mut state = TREE
            .state
            .write()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        if state.structural_epoch() != structural_epoch {
            return Err(AppError::new(
                ErrorKind::Conflict,
                "selection became stale before durable delete",
            ));
        }
        let changed_at = next_mutation_timestamp();

        let selected_album_ids = targets
            .iter()
            .filter_map(|slot_ref| state.get(slot_ref))
            .filter(|record| record.object_type == ObjectType::Album)
            .map(|record| record.id)
            .collect::<BTreeSet<_>>();
        let selected_album_members = selected_album_ids
            .iter()
            .map(|album_id| {
                (
                    *album_id,
                    state
                        .query
                        .albums
                        .get(album_id)
                        .cloned()
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();
        let affected_albums = state
            .query
            .albums
            .iter()
            .filter(|(album_id, members)| {
                !selected_album_ids.contains(*album_id)
                    && members
                        .iter()
                        .any(|ordinal| targets.ordinals().contains(ordinal))
            })
            .map(|(album_id, _)| *album_id)
            .collect::<Vec<_>>();
        let mut affected_album_patches = affected_albums
            .into_iter()
            .filter_map(|album_id| state.album_aggregate_excluding(album_id, &targets, changed_at))
            .collect::<Vec<_>>();
        for album in &mut affected_album_patches {
            album.object.touch_update_at(changed_at);
            album.metadata.last_modified_time = changed_at;
        }

        TREE.store
            .write(|writer| {
                // Album deletion removes durable reverse memberships first.
                for (album_id, members) in &selected_album_members {
                    for ordinal in members.iter() {
                        let Some(member_slot) = state.slot_for_ordinal(ordinal) else {
                            continue;
                        };
                        if targets.contains(member_slot) {
                            continue;
                        }
                        let Some(member_id) = state.get(member_slot).map(|record| record.id) else {
                            continue;
                        };
                        let Some(value) = writer.get(member_id.as_str())? else {
                            continue;
                        };
                        let mut data = value.into_value();
                        if let Some(albums) = data.albums_mut() {
                            if albums.remove(album_id) {
                                data.touch_update_at(changed_at);
                            }
                        }
                        writer.insert_at(member_id.as_str(), &data)?;
                    }
                }
                for slot_ref in targets.iter() {
                    if let Some(id) = state.get(slot_ref).map(|record| record.id) {
                        writer.remove(id.as_str())?;
                    }
                }
                for album in &affected_album_patches {
                    writer.insert_at(
                        album.object.id.as_str(),
                        &AbstractData::Album(album.clone()),
                    )?;
                }
                Ok::<(), anyhow::Error>(())
            })
            .map_err(|error| AppError::from_err(ErrorKind::Database, error))?;

        WRITE_BEHIND.cancel_targets(&targets, &selected_album_ids);

        for (album_id, members) in &selected_album_members {
            let universe = state.arena.capacity();
            state.query.edit_albums(
                members,
                &BTreeSet::new(),
                &BTreeSet::from([*album_id]),
                universe,
            );
        }
        state.remove_targets(&targets);
        for album in affected_album_patches {
            state.albums.insert(album.object.id, album);
        }
        Ok(())
    })
    .await
    .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    Ok(())
}
