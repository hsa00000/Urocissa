use crate::{
    public::{
        db::tree::TREE,
        error_data::handle_error,
        structure::{
            abstract_data::AbstractData, database_struct::{database::definition::DatabaseSchema, file_modify::FileModify},
        },
    },
    tasks::{BATCH_COORDINATOR, batcher::flush_tree::FlushTreeTask},
};
use anyhow::Result;
use arrayvec::ArrayString;
use mini_executor::Task;
use std::{path::PathBuf, time::SystemTime};
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
    type Output = Result<Option<DatabaseSchema>>;

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

fn deduplicate_task(task: DeduplicateTask) -> Result<Option<DatabaseSchema>> {
    let mut database = DatabaseSchema::new(&task.path, task.hash)?;

    // File already in persistent database

    let existing_db = TREE.load_database_from_hash(database.hash.as_str());

    if let Ok(mut database_exist) = existing_db {
        // Insert new alias into database_alias table
        let metadata = task.path.metadata()?;
        let modified = metadata.modified()?.duration_since(SystemTime::UNIX_EPOCH)?.as_millis();
        let file_modify = FileModify::new(&task.path, modified);
        let conn = TREE.get_connection()?;
        conn.execute(
            "INSERT INTO database_alias (hash, file, modified, scan_time) VALUES (?, ?, ?, ?)",
            rusqlite::params![database_exist.hash.as_str(), file_modify.file, file_modify.modified as i64, file_modify.scan_time as i64],
        )?;

        if let Some(album_id) = task.presigned_album_id_opt {
            database_exist.album.insert(album_id);
        }
        let abstract_data = AbstractData::DatabaseSchema(database_exist.into());
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
