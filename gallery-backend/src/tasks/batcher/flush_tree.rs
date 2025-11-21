use mini_executor::BatchTask;
use serde_json;

use crate::{
    public::structure::abstract_data::AbstractData,
    tasks::{BATCH_COORDINATOR, batcher::update_tree::UpdateTreeTask},
};

pub struct FlushTreeTask {
    pub insert_list: Vec<AbstractData>,
    pub remove_list: Vec<AbstractData>,
}

impl FlushTreeTask {
    pub fn insert(databases: Vec<AbstractData>) -> Self {
        Self {
            insert_list: databases,
            remove_list: Vec::new(),
        }
    }
    pub fn remove(databases: Vec<AbstractData>) -> Self {
        Self {
            insert_list: Vec::new(),
            remove_list: databases,
        }
    }
}
impl BatchTask for FlushTreeTask {
    fn batch_run(list: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            let mut all_insert_databases = Vec::new();
            let mut all_remove_databases = Vec::new();
            for task in list {
                all_insert_databases.extend(task.insert_list);
                all_remove_databases.extend(task.remove_list);
            }
            flush_tree_task(all_insert_databases, all_remove_databases);
        }
    }
}

fn flush_tree_task(insert_list: Vec<AbstractData>, remove_list: Vec<AbstractData>) {
    let conn = crate::public::db::sqlite::DB_POOL.get().unwrap();

    insert_list
        .iter()
        .for_each(|abstract_data| match abstract_data {
            AbstractData::Database(database) => {
                conn.execute(
                    "INSERT OR REPLACE INTO database (hash, size, width, height, thumbhash, phash, ext, exif_vec, tag, album, alias, ext_type, pending) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    rusqlite::params![
                        database.hash.as_str(),
                        database.size,
                        database.width,
                        database.height,
                        &database.thumbhash,
                        &database.phash,
                        &database.ext,
                        serde_json::to_string(&database.exif_vec).unwrap(),
                        serde_json::to_string(&database.tag.iter().collect::<Vec<_>>()).unwrap(),
                        serde_json::to_string(&database.album.iter().map(|a| a.as_str()).collect::<Vec<_>>()).unwrap(),
                        serde_json::to_string(&database.alias).unwrap(),
                        &database.ext_type,
                        database.pending as i32
                    ],
                ).unwrap();
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
                ).unwrap();
            }
        });

    remove_list
        .iter()
        .for_each(|abstract_data| match abstract_data {
            AbstractData::Database(database) => {
                conn.execute(
                    "DELETE FROM database WHERE hash = ?1",
                    rusqlite::params![database.hash.as_str()],
                )
                .unwrap();
            }
            AbstractData::Album(album) => {
                conn.execute(
                    "DELETE FROM album WHERE id = ?1",
                    rusqlite::params![album.id.as_str()],
                )
                .unwrap();
            }
        });

    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
}
