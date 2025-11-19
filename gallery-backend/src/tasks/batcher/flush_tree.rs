use mini_executor::BatchTask;
use rusqlite::params;

use crate::{
    public::{
        constant::DEFAULT_PRIORITY_LIST,
        db::sqlite::SQLITE,
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
    let mut conn = SQLITE.conn.lock().unwrap();
    let txn = conn.transaction().unwrap();
    {
        let mut stmt_insert_obj =
            txn.prepare("INSERT OR REPLACE INTO objects (id, data, size, width, height, ext, ext_type, pending, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)").unwrap();
        let mut stmt_insert_alb =
            txn.prepare("INSERT OR REPLACE INTO albums (id, data, title, created_time, pending, width, height) VALUES (?, ?, ?, ?, ?, ?, ?)").unwrap();
        let mut stmt_delete_obj = txn.prepare("DELETE FROM objects WHERE id = ?").unwrap();
        let mut stmt_delete_alb = txn.prepare("DELETE FROM albums WHERE id = ?").unwrap();

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
                        ]).unwrap();
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
                        ]).unwrap();
                    }
                }
            }
        }

        for abstract_data in &remove_list {
            match abstract_data {
                AbstractData::Database(database) => {
                    stmt_delete_obj.execute(params![database.hash.as_str()]).unwrap();
                }
                AbstractData::Album(album) => {
                    stmt_delete_alb.execute(params![album.id.as_str()]).unwrap();
                }
            }
        }
    }
    txn.commit().unwrap();

    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
}

