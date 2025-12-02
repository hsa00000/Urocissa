use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExifSchema {
    pub hash: String,
    pub tag: String,
    pub value: String,
}

pub struct DatabaseExifTable;

impl DatabaseExifTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS database_exif (
                hash  TEXT NOT NULL,
                tag   TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (hash, tag),
                FOREIGN KEY(hash) REFERENCES database(hash) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_database_exif_tag ON database_exif(tag);
        "#;
        conn.execute_batch(sql)?;
        Ok(())
    }
}
