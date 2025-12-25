use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::operations::transitor::index_to_hash;
use crate::public::db::tree::read_tags::TagInfo;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use rocket::serde::{Deserialize, json::Json};

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

    let vec_tags_info = tokio::task::spawn_blocking(move || -> Result<Vec<TagInfo>> {
        let data_table = open_data_table();
        let tree_snapshot = open_tree_snapshot_table(json_data.timestamp)?;

        let mut data_to_flush: Vec<AbstractData> = Vec::new();

        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;

            if let Some(guard) = data_table.get(&*hash).unwrap() {
                let mut abstract_data = guard.value();

                // Apply tag additions and removals (only regular tags)
                let tags = abstract_data.tag_mut();
                for tag in &json_data.add_tags_array {
                    tags.insert(tag.clone());
                }
                for tag in &json_data.remove_tags_array {
                    tags.remove(tag);
                }

                data_to_flush.push(abstract_data);
            }
        }

        // Flush data
        if !data_to_flush.is_empty() {
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(data_to_flush));
        }

        // Return TagInfo
        Ok(TREE_SNAPSHOT.read_tags()?)
    })
    .await
    .unwrap()?;

    // Wait for the in-memory Tree to be updated
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(Json(vec_tags_info))
}
