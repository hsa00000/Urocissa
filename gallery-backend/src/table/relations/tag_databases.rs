use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TagDatabaseSchema {
    pub hash: String,
    pub tag: String,
}

pub struct TagDatabasesTable;

impl TagDatabasesTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS tag_databases (
                hash TEXT NOT NULL,
                tag  TEXT NOT NULL,
                PRIMARY KEY (hash, tag),
                FOREIGN KEY (hash) REFERENCES database(hash) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_tag_databases_tag ON tag_databases(tag);
        "#;
        conn.execute_batch(sql)?;
        Ok(())
    }
}
