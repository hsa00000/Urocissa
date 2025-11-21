use crate::public::structure::database_struct::file_modify::FileModify;
use arrayvec::ArrayString;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
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

        let sql_create_tag_table = r#"
            CREATE TABLE IF NOT EXISTS tag_databases (
                hash TEXT NOT NULL,
                tag  TEXT NOT NULL,
                PRIMARY KEY (hash, tag),
                FOREIGN KEY (hash) REFERENCES database(hash) ON DELETE CASCADE
            );
        "#;
        conn.execute(sql_create_tag_table, [])?;

        let sql_create_index = r#"
            CREATE INDEX IF NOT EXISTS idx_tag_databases_tag ON tag_databases(tag);
        "#;
        conn.execute(sql_create_index, [])?;

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
}
