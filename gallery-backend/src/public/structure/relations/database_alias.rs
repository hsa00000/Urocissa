use rusqlite::Connection;

pub struct DatabaseAlias;

impl DatabaseAlias {
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