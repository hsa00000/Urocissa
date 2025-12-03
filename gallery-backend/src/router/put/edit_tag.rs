use crate::workflow::processors::transitor::index_to_hash;
use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;

use crate::public::structure::abstract_data::AbstractData;
use crate::table::database::MediaCombined;
use crate::table::relations::tag_databases::TagDatabaseSchema;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
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
                AbstractData::Media(media) => {
                    // 媒體的 tag 通過關聯表處理
                    let id = match media {
                        MediaCombined::Image(i) => i.object.id,
                        MediaCombined::Video(v) => v.object.id,
                    };
                    for tag in &json_data.add_tags_array {
                        flush_ops.push(FlushOperation::InsertTag(TagDatabaseSchema {
                            hash: id.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                    // 移除 tag 需要額外的處理，這裡簡化
                }
                AbstractData::Album(album) => {
                    // Apply tag additions and removals in one pass
                    for tag in &json_data.add_tags_array {
                        album.metadata.tag.insert(tag.clone());
                    }
                    for tag in &json_data.remove_tags_array {
                        album.metadata.tag.remove(tag);
                    }
                    // Flush the updated album
                    flush_ops.push(FlushOperation::InsertAbstractData(abstract_data));
                }
                AbstractData::Database(_) => {
                    // Collect tag operations
                    for tag in &json_data.add_tags_array {
                        flush_ops.push(FlushOperation::InsertTag(TagDatabaseSchema {
                            hash: hash.to_string(),
                            tag: tag.clone(),
                        }));
                    }
                    for tag in &json_data.remove_tags_array {
                        flush_ops.push(FlushOperation::RemoveTag(TagDatabaseSchema {
                            hash: hash.to_string(),
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
