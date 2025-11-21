use crate::{
    public::{
        error_data::handle_error,
        structure::{abstract_data::AbstractData, database_struct::database::definition::Database},
    },
    tasks::{BATCH_COORDINATOR, batcher::flush_tree::FlushTreeTask},
};
use anyhow::Result;
use arrayvec::ArrayString;
use mini_executor::Task;
use rusqlite::Connection;
use std::{
    mem,
    path::PathBuf,
};
use tokio::task::spawn_blocking;

pub struct DeduplicateTask {
    pub path: PathBuf,
    pub hash: ArrayString<64>,
    pub presigned_album_id_opt: Option<ArrayString<64>>,
}

impl DeduplicateTask {
    pub fn new(
        path: PathBuf,
        hash: ArrayString<64>,
        presigned_album_id_opt: Option<ArrayString<64>>,
    ) -> Self {
        Self {
            path,
            hash,
            presigned_album_id_opt,
        }
    }
}

impl Task for DeduplicateTask {
    type Output = Result<Option<Database>>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || deduplicate_task(self))
                .await
                .expect("blocking task panicked")
                // convert Err into your crate‑error via `handle_error`
                .map_err(|err| handle_error(err.context("Failed to run deduplicate task")))
        }
    }
}

fn deduplicate_task(task: DeduplicateTask) -> Result<Option<Database>> {
    let mut database = Database::new(&task.path, task.hash)?;

    let conn = crate::public::db::sqlite::DB_POOL.get().unwrap();
    // File already in persistent database

    let existing_db = conn.query_row(
        "SELECT * FROM database WHERE hash = ?",
        [database.hash.as_str()],
        Database::from_row,
    );

    if let Ok(mut database_exist) = existing_db {
        let file_modify = mem::take(&mut database.alias[0]);
        database_exist.alias.push(file_modify);
        if let Some(album_id) = task.presigned_album_id_opt {
            database_exist.album.insert(album_id);
        }
        let abstract_data = AbstractData::Database(database_exist);
        BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]));
        warn!("File already exists in the database:\n{:#?}", database);
        Ok(None)
    } else {
        if let Some(album_id) = task.presigned_album_id_opt {
            database.album.insert(album_id);
        }
        Ok(Some(database))
    }
}
