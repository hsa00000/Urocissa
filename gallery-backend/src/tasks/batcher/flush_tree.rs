use log::error;
use mini_executor::BatchTask;
use serde_json;

use crate::{
    public::db::tree::TREE,
    public::structure::abstract_data::AbstractData,
    tasks::{BATCH_COORDINATOR, batcher::update_tree::UpdateTreeTask},
};

#[derive(Debug)]
pub enum FlushOperation {
    InsertAbstractData(AbstractData),
    RemoveAbstractData(AbstractData),
    InsertTag(String, String),
    RemoveTag(String, String),
}

pub struct FlushTreeTask {
    pub operations: Vec<FlushOperation>,
}

impl FlushTreeTask {
    pub fn insert(databases: Vec<AbstractData>) -> Self {
        Self {
            operations: databases
                .into_iter()
                .map(FlushOperation::InsertAbstractData)
                .collect(),
        }
    }
    pub fn remove(databases: Vec<AbstractData>) -> Self {
        Self {
            operations: databases
                .into_iter()
                .map(FlushOperation::RemoveAbstractData)
                .collect(),
        }
    }
}
impl BatchTask for FlushTreeTask {
    fn batch_run(list: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            let mut all_operations = Vec::new();
            for task in list {
                all_operations.extend(task.operations);
            }
            if let Err(e) = flush_tree_task(all_operations) {
                error!("Error in flush_tree_task: {}", e);
            }
        }
    }
}

fn flush_tree_task(operations: Vec<FlushOperation>) -> rusqlite::Result<()> {
    let conn = TREE.get_connection().unwrap();

    for op in operations {
        match op {
            FlushOperation::InsertAbstractData(abstract_data) => match abstract_data {
                AbstractData::DatabaseSchema(database) => {
                    conn.execute(
                            "INSERT OR REPLACE INTO database (hash, size, width, height, thumbhash, phash, ext, exif_vec, album, ext_type, pending, timestamp_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                            rusqlite::params![
                                database.hash.as_str(),
                                database.size,
                                database.width,
                                database.height,
                                &database.thumbhash,
                                &database.phash,
                                &database.ext,
                                serde_json::to_string(&database.exif_vec).unwrap(),
                                serde_json::to_string(&database.album.iter().map(|a| a.as_str()).collect::<Vec<_>>()).unwrap(),
                                &database.ext_type,
                                database.pending as i32,
                                database.timestamp_ms
                            ],
                        )?;
                }
                AbstractData::Album(album) => {
                    conn.execute(
                            "INSERT OR REPLACE INTO album (id, title, created_time, start_time, end_time, last_modified_time, cover, thumbhash, user_defined_metadata, share_list, tag, width, height, item_count, item_size, pending) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                            rusqlite::params![
                                album.id.as_str(),
                                album.title,
                                album.created_time as i64,
                                album.start_time.map(|t| t as i64),
                                album.end_time.map(|t| t as i64),
                                album.last_modified_time as i64,
                                album.cover.as_ref().map(|c| c.as_str()),
                                album.thumbhash.as_ref(),
                                serde_json::to_string(&album.user_defined_metadata).unwrap(),
                                serde_json::to_string(&album.share_list).unwrap(),
                                serde_json::to_string(&album.tag.iter().collect::<Vec<_>>()).unwrap(),
                                album.width,
                                album.height,
                                album.item_count as i64,
                                album.item_size,
                                album.pending as i32
                            ],
                        )?;
                }
            },
            FlushOperation::RemoveAbstractData(abstract_data) => match abstract_data {
                AbstractData::DatabaseSchema(database) => {
                    conn.execute(
                        "DELETE FROM database WHERE hash = ?1",
                        rusqlite::params![database.hash.as_str()],
                    )?;
                }
                AbstractData::Album(album) => {
                    conn.execute(
                        "DELETE FROM album WHERE id = ?1",
                        rusqlite::params![album.id.as_str()],
                    )?;
                }
            },
            FlushOperation::InsertTag(hash, tag) => {
                conn.execute(
                    "INSERT OR IGNORE INTO tag_databases (hash, tag) VALUES (?1, ?2)",
                    rusqlite::params![hash, tag],
                )?;
            }
            FlushOperation::RemoveTag(hash, tag) => {
                conn.execute(
                    "DELETE FROM tag_databases WHERE hash = ?1 AND tag = ?2",
                    rusqlite::params![hash, tag],
                )?;
            }
        }
    }

    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
    Ok(())
}
