use crate::{
    public::{
        constant::DEFAULT_PRIORITY_LIST,
        db::tree::TREE,
        error_data::handle_error,
        structure::{
            abstract_data::AbstractData,
            database_struct::{
                database::{
                    definition::DatabaseSchema,
                    generate_timestamp::compute_timestamp_ms_by_file_modify,
                },
                file_modify::FileModify,
            },
            relations::database_alias::DatabaseAliasSchema,
        },
    },
    tasks::{
        BATCH_COORDINATOR,
        batcher::flush_tree::{FlushOperation, FlushTreeTask},
    },
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
    type Output = Result<Option<(DatabaseSchema, FlushTreeTask)>>;

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

fn deduplicate_task(task: DeduplicateTask) -> Result<Option<(DatabaseSchema, FlushTreeTask)>> {
    let mut database = DatabaseSchema::new(&task.path, task.hash)?;

    // File already in persistent database

    let existing_db = TREE.load_database_from_hash(database.hash.as_str());

    let metadata = task.path.metadata()?;
    let modified = metadata
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();
    let file_modify = FileModify::new(&task.path, modified);

    database.timestamp_ms =
        compute_timestamp_ms_by_file_modify(&file_modify, &DEFAULT_PRIORITY_LIST);

    if let Ok(mut database_exist) = existing_db {
        let mut operations = Vec::new();
        operations.push(FlushOperation::InsertDatabaseAlias(DatabaseAliasSchema {
            hash: database_exist.hash.as_str().to_string(),
            file: file_modify.file,
            modified: file_modify.modified as i64,
            scan_time: file_modify.scan_time as i64,
        }));

        if let Some(album_id) = task.presigned_album_id_opt {
            database_exist.album.insert(album_id);
        }
        operations.push(FlushOperation::InsertAbstractData(
            AbstractData::DatabaseSchema(database_exist.into()),
        ));

        BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask { operations });
        warn!("File already exists in the database:\n{:#?}", database);
        Ok(None)
    } else {
        // For new files, prepare the alias operation
        let operations = vec![FlushOperation::InsertDatabaseAlias(DatabaseAliasSchema {
            hash: database.hash.as_str().to_string(),
            file: file_modify.file,
            modified: file_modify.modified as i64,
            scan_time: file_modify.scan_time as i64,
        })];

        if let Some(album_id) = task.presigned_album_id_opt {
            database.album.insert(album_id);
        }
        Ok(Some((database, FlushTreeTask { operations })))
    }
}
