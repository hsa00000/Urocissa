use crate::operations::transitor::index_to_hash;
use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;

use crate::public::db::tree::read_tags::TagInfo;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_tags::UpdateTagsTask;
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
    let updated_data = tokio::task::spawn_blocking(move || -> Result<Vec<AbstractData>> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        let mut updated_data = Vec::new();
        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            let mut abstract_data = TREE.load_from_db(&hash)?;

            match &mut abstract_data {
                AbstractData::Album(album) => {
                    // Apply tag additions and removals in one pass
                    for tag in &json_data.add_tags_array {
                        album.tag.insert(tag.clone());
                    }
                    for tag in &json_data.remove_tags_array {
                        album.tag.remove(tag);
                    }
                }
                AbstractData::Database(database) => {
                    let conn = TREE.get_connection().unwrap();
                    // Apply tag additions
                    for tag in &json_data.add_tags_array {
                        conn.execute(
                            "INSERT OR IGNORE INTO tag_databases (hash, tag) VALUES (?1, ?2)",
                            rusqlite::params![database.hash.as_str(), tag],
                        )?;
                    }
                    // Apply tag removals
                    for tag in &json_data.remove_tags_array {
                        conn.execute(
                            "DELETE FROM tag_databases WHERE hash = ?1 AND tag = ?2",
                            rusqlite::params![database.hash.as_str(), tag],
                        )?;
                    }
                }
            }

            updated_data.push(abstract_data);
        }

        Ok(updated_data)
    })
    .await
    .unwrap()?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTagsTask::new(updated_data))
        .await
        .unwrap();

    let vec_tags_info = tokio::task::spawn_blocking(move || -> Result<Vec<TagInfo>> {
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
