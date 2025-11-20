use crate::public::structure::database_struct::file_modify::FileModify;
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use rusqlite::Connection;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Deserialize, Default, Serialize, Decode, Encode, PartialEq, Eq)]
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
        let sql = r#"
            CREATE TABLE IF NOT EXISTS database (
                hash TEXT PRIMARY KEY,
                size INTEGER,
                width INTEGER,
                height INTEGER,
                thumbhash BLOB,
                phash BLOB,
                ext TEXT,
                exif_vec TEXT,
                tag TEXT,
                album TEXT,
                alias TEXT,
                ext_type TEXT,
                pending INTEGER
            );
        "#;
        conn.execute(sql, [])?;
        Ok(())
    }
}
