use crate::operations::open_db::{open_data_and_album_tables, open_tree_snapshot_table};
use crate::process::transitor::index_to_abstract_data;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;

use crate::public::db::tree::read_tags::TagInfo;
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
        let (data_table, album_table) = open_data_and_album_tables();
        let tree_snapshot = open_tree_snapshot_table(json_data.timestamp)?;

        for &index in &json_data.index_array {
            let mut abstract_data =
                index_to_abstract_data(&tree_snapshot, &data_table, &album_table, index)?;

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
