pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Album;
use crate::public::structure::database_struct::database::definition::Database;
use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use crate::public::structure::database_struct::file_modify::FileModify;
use arrayvec::ArrayString;
use serde_json;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, LazyLock, RwLock, atomic::AtomicU64};

pub struct Tree {
    pub in_disk: Pool<SqliteConnectionManager>,
    pub in_memory: &'static Arc<RwLock<Vec<DatabaseTimestamp>>>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree::new());

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl Tree {
    pub fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        let conn = self.in_disk.get().context("Failed to get DB connection")?;
        Ok(conn)
    }
    pub fn load_from_db(&self, id: &str) -> Result<AbstractData> {
        let conn = self.get_connection()?;
        if let Ok(database) = conn.query_row(
            "SELECT * FROM database_with_tags WHERE hash = ?",
            [id],
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
                let tag_json: String = row.get("tag_json")?;
                let tag_vec: Vec<String> = serde_json::from_str(&tag_json).unwrap_or_default();
                let tag: HashSet<String> = tag_vec.into_iter().collect();
                let album_str: String = row.get("album")?;
                let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap_or_default();
                let album: HashSet<ArrayString<64>> = album_vec
                    .into_iter()
                    .filter_map(|s| ArrayString::from(&s).ok())
                    .collect();
                let alias_str: String = row.get("alias")?;
                let alias: Vec<FileModify> = serde_json::from_str(&alias_str).unwrap_or_default();
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
        ) {
            Ok(AbstractData::Database(database))
        } else if let Ok(album) =
            conn.query_row("SELECT * FROM album WHERE id = ?", [id], Album::from_row)
        {
            Ok(AbstractData::Album(album))
        } else {
            Err(anyhow::anyhow!("No data found for id: {}", id))
        }
    }

    pub fn load_all_databases_from_db(&self) -> Result<Vec<Database>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database_with_tags")?;
        let rows = stmt.query_map([], |row| {
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
            let tag_json: String = row.get("tag_json")?;
            let tag_vec: Vec<String> = serde_json::from_str(&tag_json).unwrap_or_default();
            let tag: HashSet<String> = tag_vec.into_iter().collect();
            let album_str: String = row.get("album")?;
            let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap_or_default();
            let album: HashSet<ArrayString<64>> = album_vec
                .into_iter()
                .filter_map(|s| ArrayString::from(&s).ok())
                .collect();
            let alias_str: String = row.get("alias")?;
            let alias: Vec<FileModify> = serde_json::from_str(&alias_str).unwrap_or_default();
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
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)
    }

    pub fn load_database_from_hash(&self, hash: &str) -> Result<Database> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database_with_tags WHERE hash = ?")?;
        stmt.query_row([hash], |row| {
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
            let tag_json: String = row.get("tag_json")?;
            let tag_vec: Vec<String> = serde_json::from_str(&tag_json).unwrap_or_default();
            let tag: HashSet<String> = tag_vec.into_iter().collect();
            let album_str: String = row.get("album")?;
            let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap_or_default();
            let album: HashSet<ArrayString<64>> = album_vec
                .into_iter()
                .filter_map(|s| ArrayString::from(&s).ok())
                .collect();
            let alias_str: String = row.get("alias")?;
            let alias: Vec<FileModify> = serde_json::from_str(&alias_str).unwrap_or_default();
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
        })
        .map_err(anyhow::Error::from)
    }
}
