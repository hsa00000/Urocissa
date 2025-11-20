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
use serde_json;
use std::{
    collections::{BTreeMap, HashSet},
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

    let conn = Connection::open("gallery.db").unwrap();
    // File already in persistent database

    let existing_db = conn.query_row(
        "SELECT * FROM database WHERE hash = ?",
        [database.hash.as_str()],
        |row| {
            let hash: String = row.get("hash")?;
            let size: u64 = row.get("size")?;
            let width: u32 = row.get("width")?;
            let height: u32 = row.get("height")?;
            let thumbhash: Vec<u8> = row.get("thumbhash")?;
            let phash: Vec<u8> = row.get("phash")?;
            let ext: String = row.get("ext")?;
            let exif_vec_str: String = row.get("exif_vec")?;
            let exif_vec: BTreeMap<String, String> =
                serde_json::from_str(&exif_vec_str).unwrap_or_default();
            let tag_str: String = row.get("tag")?;
            let tag: HashSet<String> = serde_json::from_str(&tag_str).unwrap_or_default();
            let album_str: String = row.get("album")?;
            let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap_or_default();
            let album: HashSet<ArrayString<64>> = album_vec
                .into_iter()
                .filter_map(|s| ArrayString::from(&s).ok())
                .collect();
            let alias_str: String = row.get("alias")?;
            let alias: Vec<crate::public::structure::database_struct::file_modify::FileModify> =
                serde_json::from_str(&alias_str).unwrap_or_default();
            let ext_type: String = row.get("ext_type")?;
            let pending: bool = row.get::<_, i32>("pending")? != 0;
            Ok(Database {
                hash: ArrayString::from(&hash).unwrap(),
                size,
                width,
                height,
                thumbhash,
                phash,
                ext,
                exif_vec,
                tag,
                album,
                alias,
                ext_type,
                pending,
            })
        },
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
