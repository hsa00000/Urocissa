use crate::public::db::tree::TREE;
use crate::public::error_data::handle_error;
use crate::public::structure::abstract_data::AbstractData;
use anyhow::Result;
use arrayvec::ArrayString;
use log::info;
use mini_executor::Task;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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
    info!("Perform album self-update (handled by DB triggers)");

    let abstract_data = TREE.load_from_db(&album_id)?;

    match abstract_data {
        AbstractData::Album(_) => {
            // Album updates are now handled by SQLite triggers on album_databases table.
            // No manual update or write-back is required here.
        }
        _ => {
            // Album has been deleted
            let ref_data = TREE.in_memory.read().unwrap();

            // Collect all data contained in this album
            let hash_list: Vec<_> = ref_data
                .par_iter()
                .filter_map(|dt| match dt {
                    AbstractData::DatabaseSchema(db) if db.album.contains(&album_id) => Some(db.hash),
                    _ => None,
                })
                .collect();

            // For each hash, load from DB, remove album_id, save back
            let conn = TREE.get_connection().unwrap();
            for hash in hash_list {
                let db_opt = TREE.load_database_from_hash(&hash).ok();
                if let Some(mut database) = db_opt {
                    database.album.remove(&album_id);
                    // Insert back
                    conn.execute(
                        "INSERT OR REPLACE INTO database (hash, size, width, height, thumbhash, phash, ext,  album, ext_type, pending, timestamp_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        rusqlite::params![
                            database.hash.as_str(),
                            database.size,
                            database.width,
                            database.height,
                            &database.thumbhash,
                            &database.phash,
                            &database.ext,
                           
                            serde_json::to_string(&database.album.iter().map(|a| a.as_str()).collect::<Vec<_>>()).unwrap(),
                            &database.ext_type,
                            database.pending as i32,
                            database.timestamp_ms
                        ],
                    ).unwrap();
                }
            }
        }
    }
    Ok(())
}
