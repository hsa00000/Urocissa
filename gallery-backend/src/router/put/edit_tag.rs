use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::workflow::processors::transitor::index_to_hash;

use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::table::relations::tag_database::TagDatabaseSchema;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::flush_tree::{FlushOperation, FlushTreeTask};
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
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
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let flush_ops = tokio::task::spawn_blocking(move || -> Result<Vec<FlushOperation>> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        let mut flush_ops = Vec::new();
        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            let mut abstract_data = TREE.load_from_db(&hash)?;

            match &mut abstract_data {
                AbstractData::Image(i) => {
                    // 圖片的 tag 通過關聯表處理
                    for tag in &json_data.add_tags_array {
                        flush_ops.push(FlushOperation::InsertTag(TagDatabaseSchema {
                            hash: i.object.id.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                    for tag in &json_data.remove_tags_array {
                        flush_ops.push(FlushOperation::RemoveTag(TagDatabaseSchema {
                            hash: i.object.id.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                }
                AbstractData::Video(v) => {
                    // 影片的 tag 通過關聯表處理
                    for tag in &json_data.add_tags_array {
                        flush_ops.push(FlushOperation::InsertTag(TagDatabaseSchema {
                            hash: v.object.id.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                    for tag in &json_data.remove_tags_array {
                        flush_ops.push(FlushOperation::RemoveTag(TagDatabaseSchema {
                            hash: v.object.id.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                }
                AbstractData::Album(album) => {
                    // 相簿現在也使用關聯表，行為與其他類型一致
                    for tag in &json_data.add_tags_array {
                        flush_ops.push(FlushOperation::InsertTag(TagDatabaseSchema {
                            hash: album.object.id.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                    for tag in &json_data.remove_tags_array {
                        flush_ops.push(FlushOperation::RemoveTag(TagDatabaseSchema {
                            hash: album.object.id.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                }
            }
        }

        Ok(flush_ops)
    })
    .await
    .unwrap()?;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask {
            operations: flush_ops,
        })
        .await
        .unwrap();

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(())
}
