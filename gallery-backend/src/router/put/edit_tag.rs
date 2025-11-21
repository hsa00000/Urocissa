use crate::operations::transitor::index_to_hash;
use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;

use crate::public::db::tree::read_tags::TagInfo;
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
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            let mut abstract_data = TREE.load_from_db(&hash)?;

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

        Ok(TREE_SNAPSHOT.read_tags()?)
    })
    .await
    .unwrap()?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(Json(vec_tags_info))
}
