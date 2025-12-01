use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DatabaseAliasSchema {
    pub hash: String,
    pub file: String,
    pub modified: i64,
    pub scan_time: i64,
}

pub struct DatabaseAliasTable;

impl DatabaseAliasTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS database_alias (
                hash TEXT NOT NULL,
                file TEXT NOT NULL,
                modified INTEGER NOT NULL,
                scan_time INTEGER NOT NULL,
                PRIMARY KEY (hash, scan_time),
                FOREIGN KEY (hash) REFERENCES database(hash) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_database_alias_scan_time ON database_alias(scan_time);
        "#;
        conn.execute_batch(sql)?;
        Ok(())
    }
}
