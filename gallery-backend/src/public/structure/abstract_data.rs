use std::collections::HashSet;

use anyhow::Result;
use arrayvec::ArrayString;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeMap;

use super::{
    album::Album,
    database_struct::{database::definition::Database, file_modify::FileModify},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbstractData {
    Database(Database),
    Album(Album),
}

impl AbstractData {
    pub fn compute_timestamp(self: &Self, priority_list: &[&str]) -> u128 {
        match self {
            AbstractData::Database(database) => database.compute_timestamp(priority_list),
            AbstractData::Album(album) => album.created_time,
        }
    }
    pub fn hash(self: &Self) -> ArrayString<64> {
        match self {
            AbstractData::Database(database) => database.hash,
            AbstractData::Album(album) => album.id,
        }
    }
    pub fn width(self: &Self) -> u32 {
        match self {
            AbstractData::Database(database) => database.width,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn height(self: &Self) -> u32 {
        match self {
            AbstractData::Database(database) => database.height,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn tag(self: &Self) -> &HashSet<String> {
        match self {
            AbstractData::Database(database) => &database.tag,
            AbstractData::Album(album) => &album.tag,
        }
    }
    pub fn tag_mut(self: &mut Self) -> &mut HashSet<String> {
        match self {
            AbstractData::Database(database) => &mut database.tag,
            AbstractData::Album(album) => &mut album.tag,
        }
    }
    pub fn load_from_db(conn: &Connection, id: &str) -> Result<Self> {
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
    pub fn load_all_databases_from_db(conn: &Connection) -> Result<Vec<Database>> {
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

    pub fn load_database_from_hash(conn: &Connection, hash: &str) -> Result<Database> {
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

impl From<Database> for AbstractData {
    fn from(database: Database) -> Self {
        AbstractData::Database(database)
    }
}

impl From<Album> for AbstractData {
    fn from(album: Album) -> Self {
        AbstractData::Album(album)
    }
}
