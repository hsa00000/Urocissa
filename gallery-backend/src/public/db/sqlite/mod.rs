use crate::public::structure::{
    album::Album, database_struct::database::definition::Database, expression::Expression,
    tag_info::TagInfo,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::sync::LazyLock;

pub mod album_items;
pub mod album_meta;
pub mod album_shares;
pub mod exif;
pub mod extensions;
pub mod nodes;
pub mod nodes_tags;
pub mod snapshots;

pub struct Sqlite {
    pub pool: Pool<SqliteConnectionManager>,
}

impl Sqlite {
    pub fn new() -> Self {
        let path = "./db/sqlite.db";
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager).expect("Failed to create pool");

        let conn = pool.get().expect("Failed to get connection");

        // Enable WAL mode for better concurrency
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA recursive_triggers = ON;",
        )
        .expect("Failed to set PRAGMA");

        // Create tables
        nodes::create_nodes_table(&conn).expect("Failed to create nodes table");
        album_meta::create_album_meta_table(&conn).expect("Failed to create album_meta table");
        album_shares::create_album_shares_table(&conn)
            .expect("Failed to create album_shares table");
        snapshots::create_snapshots_table(&conn).expect("Failed to create snapshots table");
        nodes_tags::create_node_tags_table(&conn).expect("Failed to create nodes_tags table");
        album_items::create_album_items_table(&conn).expect("Failed to create album_items table");
        extensions::create_extensions_table(&conn).expect("Failed to create extensions table");
        exif::create_exif_table(&conn).expect("Failed to create exif table");

        // Clear snapshots on startup
        conn.execute("DELETE FROM snapshots", [])
            .expect("Failed to clear snapshots");

        Self { pool }
    }

    pub fn get_database(&self, hash: &str) -> rusqlite::Result<Option<Database>> {
        let conn = self.pool.get().unwrap();
        nodes::get_database(&conn, hash)
    }

    pub fn get_album(&self, id: &str) -> rusqlite::Result<Option<Album>> {
        let conn = self.pool.get().unwrap();
        album_meta::get_album(&conn, id)
    }

    pub fn get_objects_count(&self) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let mut stmt =
            conn.prepare("SELECT COUNT(*) FROM nodes WHERE kind IN ('image', 'video')")?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn get_albums_count(&self) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM nodes WHERE kind = 'album'")?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn get_all_tags(&self) -> rusqlite::Result<Vec<TagInfo>> {
        let conn = self.pool.get().unwrap();
        nodes_tags::get_all_tags(&conn)
    }

    pub fn get_all_albums(&self) -> rusqlite::Result<Vec<Album>> {
        let conn = self.pool.get().unwrap();
        album_meta::get_all_albums(&conn)
    }

    pub fn get_album_stats(
        &self,
        album_id: &str,
    ) -> rusqlite::Result<(usize, u64, Option<u128>, Option<u128>, Option<Database>)> {
        let conn = self.pool.get().unwrap();
        album_meta::get_album_stats(&conn, album_id)
    }

    pub fn is_object_in_album(&self, object_id: &str, album_id: &str) -> rusqlite::Result<bool> {
        let conn = self.pool.get().unwrap();
        album_meta::is_object_in_album(&conn, object_id, album_id)
    }

    pub fn _get_objects_in_album(&self, album_id: &str) -> rusqlite::Result<Vec<String>> {
        let conn = self.pool.get().unwrap();
        album_items::_get_objects_in_album(&conn, album_id)
    }

    pub fn get_snapshot_len(&self, timestamp: u128) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        snapshots::get_snapshot_len(&conn, timestamp)
    }

    pub fn get_snapshot_hash(&self, timestamp: u128, idx: usize) -> rusqlite::Result<String> {
        let conn = self.pool.get().unwrap();
        snapshots::get_snapshot_hash(&conn, timestamp, idx)
    }

    pub fn get_snapshot_width_height(
        &self,
        timestamp: u128,
        idx: usize,
    ) -> rusqlite::Result<(u32, u32)> {
        let conn = self.pool.get().unwrap();
        snapshots::get_snapshot_width_height(&conn, timestamp, idx)
    }

    pub fn get_all_objects(&self) -> rusqlite::Result<Vec<Database>> {
        let conn = self.pool.get().unwrap();
        nodes::get_all_objects(&conn)
    }

    pub fn get_snapshot_dates(&self, timestamp: u128) -> rusqlite::Result<Vec<(usize, i64)>> {
        let conn = self.pool.get().unwrap();
        snapshots::get_snapshot_dates(&conn, timestamp)
    }

    pub fn generate_snapshot(
        &self,
        timestamp: u128,
        expression: &Option<Expression>,
        hide_metadata: bool,
        shared_album_id: Option<&str>,
    ) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        snapshots::generate_snapshot(&conn, timestamp, expression, hide_metadata, shared_album_id)
    }

    pub fn get_snapshot_index(
        &self,
        timestamp: u128,
        hash: &str,
    ) -> rusqlite::Result<Option<usize>> {
        let conn = self.pool.get().unwrap();
        snapshots::get_snapshot_index(&conn, timestamp, hash)
    }

    pub fn delete_expired_snapshots(&self, timestamp_threshold: u128) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        snapshots::delete_expired_snapshots(&conn, timestamp_threshold)
    }

    pub fn delete_expired_pending_data(
        &self,
        timestamp_threshold: u128,
    ) -> rusqlite::Result<(usize, usize)> {
        let conn = self.pool.get().unwrap();

        let mut stmt_obj = conn.prepare(
            "DELETE FROM nodes WHERE pending = 1 AND timestamp < ? AND kind IN ('image', 'video')",
        )?;
        let obj_count = stmt_obj.execute(params![timestamp_threshold as i64])?;

        let mut stmt_album = conn.prepare(
            "DELETE FROM nodes WHERE pending = 1 AND created_time < ? AND kind = 'album'",
        )?;
        let album_count = stmt_album.execute(params![timestamp_threshold as i64])?;

        Ok((obj_count, album_count))
    }
}

pub static SQLITE: LazyLock<Sqlite> = LazyLock::new(|| Sqlite::new());
