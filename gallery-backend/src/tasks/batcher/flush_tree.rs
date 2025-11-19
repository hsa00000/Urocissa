use mini_executor::BatchTask;
use rusqlite::params;

use crate::{
    public::{
        constant::{redb::{ALBUM_TABLE, DATA_TABLE}, DEFAULT_PRIORITY_LIST},
        db::{sqlite::SQLITE, tree::TREE},
        structure::abstract_data::AbstractData,
    },
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
    let write_txn = TREE.in_disk.begin_write().unwrap();
    {
        let mut data_table = write_txn.open_table(DATA_TABLE).unwrap();
        let mut album_table = write_txn.open_table(ALBUM_TABLE).unwrap();

        insert_list
            .iter()
            .for_each(|abstract_data| match abstract_data {
                AbstractData::Database(database) => {
                    data_table.insert(&*database.hash, database).unwrap();
                }
                AbstractData::Album(album) => {
                    album_table.insert(&*album.id, album).unwrap();
                }
            });
        remove_list
            .iter()
            .for_each(|abstract_data| match abstract_data {
                AbstractData::Database(database) => {
                    data_table.remove(&*database.hash).unwrap();
                }
                AbstractData::Album(album) => {
                    album_table.remove(&*album.id).unwrap();
                }
            });
    };
    write_txn.commit().unwrap();

    // SQLite Dual Write
    let sqlite_result = (|| -> rusqlite::Result<()> {
        let mut conn = SQLITE.conn.lock().unwrap();
        let txn = conn.transaction()?;
        {
            let mut stmt_insert_obj =
                txn.prepare("INSERT OR REPLACE INTO objects (id, data, size, width, height, ext, ext_type, pending, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")?;
            let mut stmt_insert_alb =
                txn.prepare("INSERT OR REPLACE INTO albums (id, data, title, created_time, pending, width, height) VALUES (?, ?, ?, ?, ?, ?, ?)")?;
            let mut stmt_delete_obj = txn.prepare("DELETE FROM objects WHERE id = ?")?;
            let mut stmt_delete_alb = txn.prepare("DELETE FROM albums WHERE id = ?")?;

            for abstract_data in &insert_list {
                match abstract_data {
                    AbstractData::Database(database) => {
                        if let Ok(data) = serde_json::to_vec(database) {
                            let timestamp = database.compute_timestamp(&DEFAULT_PRIORITY_LIST);
                            stmt_insert_obj.execute(params![
                                database.hash.as_str(),
                                data,
                                database.size,
                                database.width,
                                database.height,
                                database.ext,
                                database.ext_type,
                                database.pending,
                                timestamp as i64
                            ])?;
                        }
                    }
                    AbstractData::Album(album) => {
                        if let Ok(data) = serde_json::to_vec(album) {
                            stmt_insert_alb.execute(params![
                                album.id.as_str(),
                                data,
                                album.title,
                                album.created_time as i64, // SQLite supports up to 64-bit signed integers
                                album.pending,
                                album.width,
                                album.height
                            ])?;
                        }
                    }
                }
            }

            for abstract_data in &remove_list {
                match abstract_data {
                    AbstractData::Database(database) => {
                        stmt_delete_obj.execute(params![database.hash.as_str()])?;
                    }
                    AbstractData::Album(album) => {
                        stmt_delete_alb.execute(params![album.id.as_str()])?;
                    }
                }
            }
        }
        txn.commit()?;
        Ok(())
    })();

    if let Err(e) = sqlite_result {
        log::error!("SQLite Dual Write Failed: {}", e);
    }

    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
}
