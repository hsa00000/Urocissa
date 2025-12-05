use log::error;
use mini_executor::BatchTask;
use serde_json;

use crate::{
    public::db::tree::TREE,
    public::structure::abstract_data::AbstractData,
    table::relations::album_database::AlbumDatabaseSchema,
    table::relations::database_alias::DatabaseAliasSchema,
    table::relations::database_exif::ExifSchema,
    table::relations::tag_database::TagDatabaseSchema,
    workflow::tasks::{BATCH_COORDINATOR, batcher::update_tree::UpdateTreeTask},
};

fn get_file_from_hash(hash: &str) -> Option<String> {
    let conn = TREE.get_connection().unwrap();
    let mut stmt = conn
        .prepare("SELECT file FROM database_alias WHERE hash = ?")
        .ok()?;
    let mut rows = stmt.query(rusqlite::params![hash]).ok()?;
    if let Some(row) = rows.next().ok()? {
        Some(row.get::<_, String>(0).ok()?)
    } else {
        None
    }
}

#[derive(Debug)]
pub enum FlushOperation {
    InsertAbstractData(AbstractData),
    RemoveAbstractData(AbstractData),
    InsertTag(TagDatabaseSchema),
    RemoveTag(TagDatabaseSchema),
    InsertAlbum(AlbumDatabaseSchema),
    RemoveAlbum(AlbumDatabaseSchema),
    InsertDatabaseAlias(DatabaseAliasSchema),
    InsertExif(ExifSchema),
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
                AbstractData::Image(img) => {
                    // 1. Object 表：無腦 Upsert
                    // 無論是新插入，還是影片轉圖片更新，或是 Reindex，都用這招
                    tx.execute(
                        "INSERT INTO object (id, obj_type, created_time, pending, thumbhash) 
                         VALUES (?1, 'image', ?2, ?3, ?4)
                         ON CONFLICT(id) DO UPDATE SET 
                         obj_type=excluded.obj_type,
                         created_time=excluded.created_time,
                         pending=excluded.pending,
                         thumbhash=excluded.thumbhash",
                        rusqlite::params![
                            img.object.id.as_str(),
                            img.object.created_time,
                            img.object.pending as i32,
                            img.object.thumbhash
                        ],
                    )?;

                    // 2. Meta Image 表：無腦 Upsert
                    tx.execute(
                        "INSERT INTO meta_image (id, size, width, height, ext, phash) 
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                         ON CONFLICT(id) DO UPDATE SET 
                         size=excluded.size,
                         width=excluded.width,
                         height=excluded.height,
                         ext=excluded.ext,
                         phash=excluded.phash",
                        rusqlite::params![
                            img.object.id.as_str(),
                            img.metadata.size,
                            img.metadata.width,
                            img.metadata.height,
                            img.metadata.ext,
                            img.metadata.phash
                        ],
                    )?;

                    // 3. 同步相簿關聯 (先刪後加，確保與記憶體狀態一致)
                    tx.execute(
                        "DELETE FROM album_database WHERE hash = ?1",
                        rusqlite::params![img.object.id.as_str()],
                    )?;
                    for album_id in &img.albums {
                        tx.execute(
                            "INSERT OR IGNORE INTO album_database (album_id, hash) VALUES (?1, ?2)",
                            rusqlite::params![album_id.as_str(), img.object.id.as_str()],
                        )?;
                    }
                }
                AbstractData::Video(vid) => {
                    // 1. Object 表：無腦 Upsert (解決 pending=true 變 false 的衝突)
                    tx.execute(
                        "INSERT INTO object (id, obj_type, created_time, pending, thumbhash) 
                         VALUES (?1, 'video', ?2, ?3, ?4)
                         ON CONFLICT(id) DO UPDATE SET 
                         obj_type=excluded.obj_type,
                         created_time=excluded.created_time,
                         pending=excluded.pending,
                         thumbhash=excluded.thumbhash",
                        rusqlite::params![
                            vid.object.id.as_str(),
                            vid.object.created_time,
                            vid.object.pending as i32,
                            vid.object.thumbhash
                        ],
                    )?;

                    // 2. Meta Video 表：無腦 Upsert
                    tx.execute(
                        "INSERT INTO meta_video (id, size, width, height, ext, duration) 
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                         ON CONFLICT(id) DO UPDATE SET 
                         size=excluded.size,
                         width=excluded.width,
                         height=excluded.height,
                         ext=excluded.ext,
                         duration=excluded.duration",
                        rusqlite::params![
                            vid.object.id.as_str(),
                            vid.metadata.size,
                            vid.metadata.width,
                            vid.metadata.height,
                            vid.metadata.ext,
                            vid.metadata.duration
                        ],
                    )?;

                    // 3. 同步相簿關聯
                    tx.execute(
                        "DELETE FROM album_database WHERE hash = ?1",
                        rusqlite::params![vid.object.id.as_str()],
                    )?;
                    for album_id in &vid.albums {
                        tx.execute(
                            "INSERT OR IGNORE INTO album_database (album_id, hash) VALUES (?1, ?2)",
                            rusqlite::params![album_id.as_str(), vid.object.id.as_str()],
                        )?;
                    }
                }

                AbstractData::Album(album) => {
                    // Album Object: Upsert
                    tx.execute(
                        "INSERT INTO object (id, obj_type, created_time, pending, thumbhash) \
                         VALUES (?1, 'album', ?2, ?3, ?4) \
                         ON CONFLICT(id) DO UPDATE SET \
                         obj_type=excluded.obj_type, \
                         created_time=excluded.created_time, \
                         pending=excluded.pending, \
                         thumbhash=excluded.thumbhash",
                        rusqlite::params![
                            album.object.id.as_str(),
                            album.object.created_time,
                            album.object.pending as i32,
                            album.object.thumbhash,
                        ],
                    )?;

                    // Meta Album: Upsert
                    tx.execute(
                        "INSERT INTO meta_album \
                         (id, title, start_time, end_time, last_modified_time, \
                          cover, user_defined_metadata, \
                          item_count, item_size) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                         ON CONFLICT(id) DO UPDATE SET \
                         title=excluded.title, \
                         start_time=excluded.start_time, \
                         end_time=excluded.end_time, \
                         last_modified_time=excluded.last_modified_time, \
                         cover=excluded.cover, \
                         user_defined_metadata=excluded.user_defined_metadata, \
                         item_count=excluded.item_count, \
                         item_size=excluded.item_size",
                        rusqlite::params![
                            album.object.id.as_str(),
                            album.metadata.title,
                            album.metadata.start_time.map(|t| t as i64),
                            album.metadata.end_time.map(|t| t as i64),
                            album.metadata.last_modified_time as i64,
                            album.metadata.cover.as_ref().map(|c| c.as_str()),
                            serde_json::to_string(&album.metadata.user_defined_metadata).unwrap(),
                            album.metadata.item_count as i64,
                            album.metadata.item_size,
                        ],
                    )?;
                }
            },
            FlushOperation::RemoveAbstractData(abstract_data) => match abstract_data {
                AbstractData::Image(i) => {
                    tx.execute(
                        "DELETE FROM object WHERE id = ?1",
                        rusqlite::params![i.object.id.as_str()],
                    )?;
                }
                AbstractData::Video(v) => {
                    tx.execute(
                        "DELETE FROM object WHERE id = ?1",
                        rusqlite::params![v.object.id.as_str()],
                    )?;
                }

                AbstractData::Album(album) => {
                    tx.execute(
                        "DELETE FROM object WHERE id = ?1",
                        rusqlite::params![album.object.id.as_str()],
                    )?;
                }
            },
            FlushOperation::InsertTag(schema) => {
                tx.execute(
                    "INSERT OR IGNORE INTO tag_database (hash, tag) VALUES (?1, ?2)",
                    rusqlite::params![schema.hash, schema.tag],
                )?;
            }
            FlushOperation::RemoveTag(schema) => {
                tx.execute(
                    "DELETE FROM tag_database WHERE hash = ?1 AND tag = ?2",
                    rusqlite::params![schema.hash, schema.tag],
                )?;
            }
            FlushOperation::InsertAlbum(schema) => {
                tx.execute(
                    "INSERT OR IGNORE INTO album_database (album_id, hash) VALUES (?1, ?2)",
                    rusqlite::params![schema.album_id, schema.hash],
                )?;
            }
            FlushOperation::RemoveAlbum(schema) => {
                tx.execute(
                    "DELETE FROM album_database WHERE album_id = ?1 AND hash = ?2",
                    rusqlite::params![schema.album_id, schema.hash],
                )?;
            }
            FlushOperation::InsertDatabaseAlias(schema) => {
                // 這裡原本就是 Upsert，保持原樣
                tx.execute(
                    "INSERT INTO database_alias (hash, file, modified, scan_time) \
                     VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT(hash, scan_time) DO UPDATE SET \
                     file=excluded.file, \
                     modified=excluded.modified, \
                     scan_time=excluded.scan_time",
                    rusqlite::params![schema.hash, schema.file, schema.modified, schema.scan_time],
                )?;
            }
            FlushOperation::InsertExif(schema) => {
                // 這裡原本就是 REPLACE (也是一種 Upsert)，保持原樣
                if let Err(e) = tx.execute(
                    "INSERT OR REPLACE INTO database_exif (hash, tag, value) VALUES (?1, ?2, ?3)",
                    rusqlite::params![schema.hash, schema.tag, schema.value],
                ) {
                    return Err(e);
                }
            }
        }
    }

    tx.commit()?;
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
    Ok(())
}
