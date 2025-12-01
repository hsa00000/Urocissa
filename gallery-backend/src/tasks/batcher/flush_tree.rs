use log::error;
use mini_executor::BatchTask;
use serde_json;

use crate::{
    public::db::tree::TREE,
    public::structure::abstract_data::AbstractData,
    table::relations::database_alias::DatabaseAliasSchema,
    table::relations::database_exif::ExifSchema,
    table::relations::tag_databases::TagDatabaseSchema,
    tasks::{BATCH_COORDINATOR, batcher::update_tree::UpdateTreeTask},
};

#[derive(Debug)]
pub enum FlushOperation {
    InsertAbstractData(AbstractData),
    RemoveAbstractData(AbstractData),
    InsertTag(TagDatabaseSchema),
    RemoveTag(TagDatabaseSchema),
    InsertDatabaseAlias(DatabaseAliasSchema),
    InsertExif(ExifSchema),
    RemoveExif(ExifSchema),
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
    let mut conn = TREE.get_connection().unwrap();

    // 開一個 transaction，把整批 operations 包起來
    let tx = conn.transaction()?;

    for op in operations {
        match op {
            FlushOperation::InsertAbstractData(abstract_data) => match abstract_data {
                AbstractData::DatabaseSchema(database) => {
                    tx.execute(
                        "INSERT INTO database \
                         (hash, size, width, height, thumbhash, phash, ext, album, \
                          ext_type, pending, timestamp_ms) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
                         ON CONFLICT(hash) DO UPDATE SET \
                         size=excluded.size, \
                         width=excluded.width, \
                         height=excluded.height, \
                         thumbhash=excluded.thumbhash, \
                         phash=excluded.phash, \
                         ext=excluded.ext, \
                         album=excluded.album, \
                         ext_type=excluded.ext_type, \
                         pending=excluded.pending, \
                         timestamp_ms=excluded.timestamp_ms",
                        rusqlite::params![
                            database.hash.as_str(),
                            database.size,
                            database.width,
                            database.height,
                            &database.thumbhash,
                            &database.phash,
                            &database.ext,
                            serde_json::to_string(
                                &database
                                    .album
                                    .iter()
                                    .map(|a| a.as_str())
                                    .collect::<Vec<_>>()
                            )
                            .unwrap(),
                            &database.ext_type,
                            database.pending as i32,
                            database.timestamp_ms,
                        ],
                    )?;

                    // Sync album_databases to ensure triggers fire
                    // 1. Remove old relationships for this file
                    tx.execute(
                        "DELETE FROM album_databases WHERE hash = ?1",
                        rusqlite::params![database.hash.as_str()],
                    )?;

                    // 2. Insert new relationships
                    for album_id in &database.album {
                        tx.execute(
                            "INSERT OR IGNORE INTO album_databases (album_id, hash) VALUES (?1, ?2)",
                            rusqlite::params![album_id.as_str(), database.hash.as_str()],
                        )?;
                    }
                }
                AbstractData::Album(album) => {
                    tx.execute(
                        "INSERT INTO album \
                         (id, title, created_time, start_time, end_time, last_modified_time, \
                          cover, thumbhash, user_defined_metadata, tag, \
                          item_count, item_size, pending) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
                         ON CONFLICT(id) DO UPDATE SET \
                         title=excluded.title, \
                         created_time=excluded.created_time, \
                         start_time=excluded.start_time, \
                         end_time=excluded.end_time, \
                         last_modified_time=excluded.last_modified_time, \
                         cover=excluded.cover, \
                         thumbhash=excluded.thumbhash, \
                         user_defined_metadata=excluded.user_defined_metadata, \
                         tag=excluded.tag, \
                         item_count=excluded.item_count, \
                         item_size=excluded.item_size, \
                         pending=excluded.pending",
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
                            serde_json::to_string(&album.tag.iter().collect::<Vec<_>>()).unwrap(),
                            album.item_count as i64,
                            album.item_size,
                            album.pending as i32,
                        ],
                    )?;
                }
            },
            FlushOperation::RemoveAbstractData(abstract_data) => match abstract_data {
                AbstractData::DatabaseSchema(database) => {
                    tx.execute(
                        "DELETE FROM database WHERE hash = ?1",
                        rusqlite::params![database.hash.as_str()],
                    )?;
                    // Also remove from album_databases to trigger updates
                    tx.execute(
                        "DELETE FROM album_databases WHERE hash = ?1",
                        rusqlite::params![database.hash.as_str()],
                    )?;
                }
                AbstractData::Album(album) => {
                    tx.execute(
                        "DELETE FROM album WHERE id = ?1",
                        rusqlite::params![album.id.as_str()],
                    )?;
                }
            },
            FlushOperation::InsertTag(schema) => {
                tx.execute(
                    "INSERT OR IGNORE INTO tag_databases (hash, tag) VALUES (?1, ?2)",
                    rusqlite::params![schema.hash, schema.tag],
                )?;
            }
            FlushOperation::RemoveTag(schema) => {
                tx.execute(
                    "DELETE FROM tag_databases WHERE hash = ?1 AND tag = ?2",
                    rusqlite::params![schema.hash, schema.tag],
                )?;
            }
            FlushOperation::InsertDatabaseAlias(schema) => {
                tx.execute(
                    "INSERT INTO database_alias \
                     (hash, file, modified, scan_time) \
                     VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT(hash, scan_time) DO UPDATE SET \
                     file=excluded.file, \
                     modified=excluded.modified, \
                     scan_time=excluded.scan_time",
                    rusqlite::params![schema.hash, schema.file, schema.modified, schema.scan_time],
                )?;
            }
            FlushOperation::InsertExif(schema) => {
                if let Err(e) = tx.execute(
                    "INSERT OR REPLACE INTO database_exif (hash, tag, value) VALUES (?1, ?2, ?3)",
                    rusqlite::params![schema.hash, schema.tag, schema.value],
                ) {
                    return Err(e);
                }
            }
            FlushOperation::RemoveExif(schema) => {
                tx.execute(
                    "DELETE FROM database_exif WHERE hash = ?1 AND tag = ?2",
                    rusqlite::params![schema.hash, schema.tag],
                )?;
            }
        }
    }

    // 全部 SQL 都成功之後才 commit
    tx.commit()?;

    // 確定已經寫入 DB，再更新樹
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);

    Ok(())
}
