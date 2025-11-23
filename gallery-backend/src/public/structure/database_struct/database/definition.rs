use crate::public::structure::database_struct::file_modify::FileModify;
use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
pub struct Database {
    pub hash: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub thumbhash: Vec<u8>,
    pub phash: Vec<u8>,
    pub ext: String,
    pub exif_vec: BTreeMap<String, String>,
    pub tag: HashSet<String>,
    pub album: HashSet<ArrayString<64>>,
    pub alias: Vec<FileModify>,
    pub ext_type: String,
    pub pending: bool,
}

impl Database {
    pub fn create_database_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql_create_main_table = r#"
            CREATE TABLE IF NOT EXISTS database (
                hash TEXT PRIMARY KEY,
                size INTEGER,
                width INTEGER,
                height INTEGER,
                thumbhash BLOB,
                phash BLOB,
                ext TEXT,
                exif_vec TEXT,
                album TEXT,
                alias TEXT,
                ext_type TEXT,
                pending INTEGER
            );
        "#;
        conn.execute(sql_create_main_table, [])?;

        let sql_create_view = r#"
            CREATE VIEW IF NOT EXISTS database_with_tags AS
            SELECT
                d.*,
                COALESCE(
                  (SELECT json_group_array(td.tag)
                     FROM tag_databases td
                    WHERE td.hash = d.hash
                  ),
                  '[]'
                ) AS tag_json
            FROM database d;
        "#;
        conn.execute(sql_create_view, [])?;

        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let hash: String = row.get("hash")?;
        let size: u64 = row.get("size")?;
        let width: u32 = row.get("width")?;
        let height: u32 = row.get("height")?;
        let thumbhash: Vec<u8> = row.get("thumbhash")?;
        let phash: Vec<u8> = row.get("phash")?;
        let ext: String = row.get("ext")?;
        let exif_vec_str: String = row.get("exif_vec")?;
        let exif_vec: BTreeMap<String, String> = serde_json::from_str(&exif_vec_str).unwrap();
        let tag_json: String = row.get("tag_json")?;
        let tag_vec: Vec<String> = serde_json::from_str(&tag_json).unwrap();
        let tag: HashSet<String> = tag_vec.into_iter().collect();
        let album_str: String = row.get("album")?;
        let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap();
        let album: HashSet<ArrayString<64>> = album_vec
            .into_iter()
            .filter_map(|s| ArrayString::from(&s).ok())
            .collect();
        let alias_str: String = row.get("alias")?;
        let alias: Vec<FileModify> = serde_json::from_str(&alias_str).unwrap();
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
    }

    pub fn from_row_basic(row: &Row) -> rusqlite::Result<Self> {
        let hash: String = row.get("hash")?;
        let size: u64 = row.get("size")?;
        let width: u32 = row.get("width")?;
        let height: u32 = row.get("height")?;
        let thumbhash: Vec<u8> = row.get("thumbhash")?;
        let phash: Vec<u8> = row.get("phash")?;
        let ext: String = row.get("ext")?;

        let exif_vec_str: String = row.get("exif_vec")?;
        let exif_vec: BTreeMap<String, String> = serde_json::from_str(&exif_vec_str).unwrap();

        let album_str: String = row.get("album")?;
        let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap();
        let album: HashSet<ArrayString<64>> = album_vec
            .into_iter()
            .filter_map(|s| ArrayString::from(&s).ok())
            .collect();

        let alias_str: String = row.get("alias")?;
        let alias: Vec<FileModify> = serde_json::from_str(&alias_str).unwrap();

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
            tag: HashSet::new(), // tags 後面累積
            album,
            alias,
            ext_type,
            pending,
        })
    }

    pub fn load_databases_with_tags(conn: &Connection) -> rusqlite::Result<Vec<Database>> {
        let sql = r#"
            SELECT
                database.hash,
                database.size,
                database.width,
                database.height,
                database.thumbhash,
                database.phash,
                database.ext,
                database.exif_vec,
                database.album,
                database.alias,
                database.ext_type,
                database.pending,
                tag_databases.tag AS tag
            FROM database
            LEFT JOIN tag_databases
                ON tag_databases.hash = database.hash
            ORDER BY database.hash;
        "#;

        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query([])?;

        let mut result: Vec<Database> = Vec::new();
        let mut current_hash: Option<String> = None;

        while let Some(row) = rows.next()? {
            let hash: String = row.get("hash")?;
            let tag_opt: Option<String> = row.get("tag")?;

            if current_hash.as_deref() != Some(&hash) {
                let mut db = Database::from_row_basic(row)?;

                if let Some(tag) = tag_opt {
                    db.tag.insert(tag);
                }

                result.push(db);
                current_hash = Some(hash);
            } else {
                if let Some(tag) = tag_opt {
                    if let Some(last) = result.last_mut() {
                        last.tag.insert(tag);
                    }
                }
            }
        }

        Ok(result)
    }
}
