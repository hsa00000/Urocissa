use crate::public::structure::database_struct::file_modify::FileModify;
use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// 新的：單表版本（沒有 tag）
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

        // 先保留舊 view，避免既有依賴炸裂
        let sql_create_view = r#"
            CREATE VIEW IF NOT EXISTS database_with_tags AS
            SELECT
                database.*,
                COALESCE(
                  (SELECT json_group_array(tag_databases.tag)
                     FROM tag_databases
                    WHERE tag_databases.hash = database.hash
                  ),
                  '[]'
                ) AS tag_json
            FROM database;
        "#;
        conn.execute(sql_create_view, [])?;

        Ok(())
    }

    /// 新的 from_row：只解析單表欄位（原 from_row_basic 改名移過來）
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
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
            album,
            alias,
            ext_type,
            pending,
        })
    }

    /// 單表載入（給未來慢慢迁移用）
    pub fn load_databases(conn: &Connection) -> rusqlite::Result<Vec<Database>> {
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
                database.pending
            FROM database
            ORDER BY database.hash;
        "#;

        let mut stmt = conn.prepare(sql)?;
        let iter = stmt.query_map([], |row| Database::from_row(row))?;

        let mut result = Vec::new();
        for db in iter {
            result.push(db?);
        }
        Ok(result)
    }
}

/// 舊的：含 tag 版本（原 Database 改名）
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
pub struct DatabaseWithTag {
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

impl From<Database> for DatabaseWithTag {
    fn from(db: Database) -> Self {
        DatabaseWithTag {
            hash: db.hash,
            size: db.size,
            width: db.width,
            height: db.height,
            thumbhash: db.thumbhash,
            phash: db.phash,
            ext: db.ext,
            exif_vec: db.exif_vec,
            tag: HashSet::new(),
            album: db.album,
            alias: db.alias,
            ext_type: db.ext_type,
            pending: db.pending,
        }
    }
}

impl DatabaseWithTag {
    /// 舊的 from_row（吃 tag_json）原封不動搬過來
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
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
        let tag: HashSet<String> = serde_json::from_str::<Vec<String>>(&tag_json)
            .unwrap_or_default()
            .into_iter()
            .collect();

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

        Ok(DatabaseWithTag {
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

    /// 舊的 load_databases_with_tags：保留行為，但型別改成 DatabaseWithTag
    pub fn load_databases_with_tags(conn: &Connection) -> rusqlite::Result<Vec<DatabaseWithTag>> {
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

        let mut result: Vec<DatabaseWithTag> = Vec::new();
        let mut current_hash: Option<String> = None;

        while let Some(row) = rows.next()? {
            let hash: String = row.get("hash")?;
            let tag_opt: Option<String> = row.get("tag")?;

            if current_hash.as_deref() != Some(&hash) {
                // 先用新的單表 from_row 解析
                let base = Database::from_row(row)?;
                let mut db_with_tag: DatabaseWithTag = base.into();

                if let Some(tag) = tag_opt {
                    db_with_tag.tag.insert(tag);
                }

                result.push(db_with_tag);
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
