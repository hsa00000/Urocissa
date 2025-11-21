use crate::public::db::tree::TREE;
use crate::public::error_data::handle_error;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Album;
use crate::public::structure::database_struct::database::definition::Database;
use anyhow::Result;
use arrayvec::ArrayString;
use log::info;
use mini_executor::Task;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::Connection;
use serde_json;
use tokio::task::spawn_blocking;

pub struct AlbumSelfUpdateTask {
    album_id: ArrayString<64>,
}

impl AlbumSelfUpdateTask {
    pub fn new(album_id: ArrayString<64>) -> Self {
        Self { album_id }
    }
}

impl Task for AlbumSelfUpdateTask {
    type Output = Result<()>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || album_task(self.album_id))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run album task")))
        }
    }
}

pub fn album_task(album_id: ArrayString<64>) -> Result<()> {
    info!("Perform album self-update");

    let conn = crate::public::db::sqlite::DB_POOL.get().unwrap();

    let album_opt = conn
        .query_row(
            "SELECT * FROM album WHERE id = ?",
            [album_id.as_str()],
            Album::from_row,
        )
        .ok();

    match album_opt {
        Some(mut album) => {
            album.pending = true;
            album.self_update();
            album.pending = false;
            // Insert back
            Album::create_album_table(&conn).unwrap(); // Ensure table exists, but probably already does
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
        _ => {
            // Album has been deleted
            let ref_data = TREE.in_memory.read().unwrap();

            // Collect all data contained in this album
            let hash_list: Vec<_> = ref_data
                .par_iter()
                .filter_map(|dt| match &dt.abstract_data {
                    AbstractData::Database(db) if db.album.contains(&album_id) => Some(db.hash),
                    _ => None,
                })
                .collect();

            // For each hash, load from DB, remove album_id, save back
            for hash in hash_list {
                let db_opt = conn
                    .query_row(
                        "SELECT * FROM database WHERE hash = ?",
                        [hash.as_str()],
                        Database::from_row,
                    )
                    .ok();
                if let Some(mut database) = db_opt {
                    database.album.remove(&album_id);
                    // Insert back
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
            }
        }
    }
    Ok(())
}
