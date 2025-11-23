use rusqlite::Connection;

pub struct TagDatabases;

impl TagDatabases {
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
