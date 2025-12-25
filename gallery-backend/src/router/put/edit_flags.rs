use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::operations::transitor::index_to_hash;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::actor::album::AlbumSelfUpdateTask;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use arrayvec::ArrayString;
use rocket::serde::{Deserialize, json::Json};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditFlagsData {
    index_array: Vec<usize>,
    timestamp: u128,
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
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<EditFlagsData>,
) -> AppResult<Json<()>> {
    let _ = auth?;
    let _ = read_only_mode?;

    // Check if trashed flag is being modified
    let is_trashed_involved = json_data.is_trashed.is_some();

    let affected_album_ids = tokio::task::spawn_blocking(
        move || -> Result<HashSet<ArrayString<64>>> {
            let data_table = open_data_table();
            let tree_snapshot = open_tree_snapshot_table(json_data.timestamp)?;

            let mut affected_album_ids = HashSet::new();
            let mut data_to_flush: Vec<AbstractData> = Vec::new();

            for &index in &json_data.index_array {
                let hash = index_to_hash(&tree_snapshot, index)?;

                if let Some(guard) = data_table.get(&*hash).unwrap() {
                    let mut abstract_data = guard.value();

                    // If trashed is involved, record the albums this data belongs to
                    if is_trashed_involved {
                        if let Some(albums) = abstract_data.albums() {
                            for album_id in albums {
                                affected_album_ids.insert(album_id.clone());
                            }
                        }
                    }

                    // Apply flag changes
                    if let Some(is_favorite) = json_data.is_favorite {
                        abstract_data.set_favorite(is_favorite);
                    }
                    if let Some(is_archived) = json_data.is_archived {
                        abstract_data.set_archived(is_archived);
                    }
                    if let Some(is_trashed) = json_data.is_trashed {
                        abstract_data.set_trashed(is_trashed);
                    }

                    data_to_flush.push(abstract_data);
                }
            }

            // Flush data
            if !data_to_flush.is_empty() {
                BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(data_to_flush));
            }

            Ok(affected_album_ids)
        },
    )
    .await
    .unwrap()?;

    // Wait for the in-memory Tree to be updated
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    // After memory update, trigger album self-update
    if !affected_album_ids.is_empty() {
        for album_id in affected_album_ids {
            BATCH_COORDINATOR.execute_detached(AlbumSelfUpdateTask::new(album_id));
        }
    }

    Ok(Json(()))
}
