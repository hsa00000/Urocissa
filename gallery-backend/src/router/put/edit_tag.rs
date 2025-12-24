use crate::operations::open_db::{open_data_and_album_tables, open_tree_snapshot_table};
use crate::process::transitor::index_to_abstract_data;
use crate::public::db::tree::read_tags::TagInfo;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::actor::album::AlbumSelfUpdateTask; // Adjust the path as per your setup
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use arrayvec::ArrayString;
use rocket::serde::{Deserialize, json::Json};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditTagsData {
    index_array: Vec<usize>,
    add_tags_array: Vec<String>,
    remove_tags_array: Vec<String>,
    timestamp: u128,
}

#[put("/put/edit_tag", format = "json", data = "<json_data>")]
pub async fn edit_tag(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<EditTagsData>,
) -> AppResult<Json<Vec<TagInfo>>> {
    let _ = auth?;
    let _ = read_only_mode?;

    // Check if this operation involves the _trashed tag
    let is_trashed_involved = json_data.add_tags_array.contains(&"_trashed".to_string())
        || json_data
            .remove_tags_array
            .contains(&"_trashed".to_string());

    // Modify return type to also return a HashSet<ArrayString<64>> containing affected album IDs
    let (vec_tags_info, affected_album_ids) = tokio::task::spawn_blocking(
        move || -> Result<(Vec<TagInfo>, HashSet<ArrayString<64>>)> {
            let (data_table, album_table) = open_data_and_album_tables();
            let tree_snapshot = open_tree_snapshot_table(json_data.timestamp)?;

            // Collect affected album IDs
            let mut affected_album_ids = HashSet::new();

            for &index in &json_data.index_array {
                let mut abstract_data =
                    index_to_abstract_data(&tree_snapshot, &data_table, &album_table, index)?;

                // If _trashed is involved and the data is a photo (Database), record its albums
                if is_trashed_involved {
                    if let AbstractData::Database(db) = &abstract_data {
                        for album_id in &db.album {
                            affected_album_ids.insert(album_id.clone());
                        }
                    }
                }

                let tag_set = abstract_data.tag_mut();

                // Apply tag additions and removals in one pass
                for tag in &json_data.add_tags_array {
                    tag_set.insert(tag.clone());
                }
                for tag in &json_data.remove_tags_array {
                    tag_set.remove(tag);
                }

                BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]))
            }

            // Return TagInfo and affected album IDs
            Ok((TREE_SNAPSHOT.read_tags()?, affected_album_ids))
        },
    )
    .await
    .unwrap()?; // Handle JoinError and Result

    // Wait for the in-memory Tree to be updated
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    // After memory update, trigger album self-update to ensure reading the latest state (excluding photos marked as trashed)
    if !affected_album_ids.is_empty() {
        for album_id in affected_album_ids {
            BATCH_COORDINATOR.execute_detached(AlbumSelfUpdateTask::new(album_id));
        }
    }

    Ok(Json(vec_tags_info))
}
