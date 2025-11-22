use rusqlite::Connection;

pub struct AlbumDatabases;

impl AlbumDatabases {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS album_databases (
                album_id TEXT NOT NULL,
                hash     TEXT NOT NULL,
                PRIMARY KEY (album_id, hash),
                FOREIGN KEY (album_id) REFERENCES album(id) ON DELETE CASCADE,
                FOREIGN KEY (hash)     REFERENCES database(hash) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_album_databases_album_id
                ON album_databases(album_id);

            CREATE INDEX IF NOT EXISTS idx_album_databases_hash
                ON album_databases(hash);
        "#;
        conn.execute_batch(sql)?;
        Ok(())
    }
}
