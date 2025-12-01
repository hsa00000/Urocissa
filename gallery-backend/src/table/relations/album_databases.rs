use rusqlite::Connection;

pub struct AlbumDatabasesTable;

impl AlbumDatabasesTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r##"
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

            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_insert AFTER INSERT ON album_databases
            BEGIN
                UPDATE album SET
                    item_count = (SELECT COUNT(*) FROM album_databases WHERE album_id = NEW.album_id),
                    item_size = (
                        SELECT COALESCE(SUM(d.size), 0)
                        FROM album_databases ad
                        JOIN database d ON ad.hash = d.hash
                        WHERE ad.album_id = NEW.album_id
                    ),
                    start_time = (
                        SELECT MIN(d.timestamp_ms)
                        FROM album_databases ad
                        JOIN database d ON ad.hash = d.hash
                        WHERE ad.album_id = NEW.album_id
                    ),
                    end_time = (
                        SELECT MAX(d.timestamp_ms)
                        FROM album_databases ad
                        JOIN database d ON ad.hash = d.hash
                        WHERE ad.album_id = NEW.album_id
                    ),
                    cover = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_databases WHERE album_id = NEW.album_id AND hash = cover) THEN cover
                        ELSE (
                            SELECT d.hash
                            FROM album_databases ad
                            JOIN database d ON ad.hash = d.hash
                            WHERE ad.album_id = NEW.album_id
                            ORDER BY d.timestamp_ms ASC
                            LIMIT 1
                        )
                    END,
                    thumbhash = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_databases WHERE album_id = NEW.album_id AND hash = cover) THEN thumbhash
                        ELSE (
                            SELECT d.thumbhash
                            FROM album_databases ad
                            JOIN database d ON ad.hash = d.hash
                            WHERE ad.album_id = NEW.album_id
                            ORDER BY d.timestamp_ms ASC
                            LIMIT 1
                        )
                    END,
                    last_modified_time = (strftime('%s', 'now') * 1000)
                WHERE id = NEW.album_id;
            END;

            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_delete AFTER DELETE ON album_databases
            BEGIN
                UPDATE album SET
                    item_count = (SELECT COUNT(*) FROM album_databases WHERE album_id = OLD.album_id),
                    item_size = (
                        SELECT COALESCE(SUM(d.size), 0)
                        FROM album_databases ad
                        JOIN database d ON ad.hash = d.hash
                        WHERE ad.album_id = OLD.album_id
                    ),
                    start_time = (
                        SELECT MIN(d.timestamp_ms)
                        FROM album_databases ad
                        JOIN database d ON ad.hash = d.hash
                        WHERE ad.album_id = OLD.album_id
                    ),
                    end_time = (
                        SELECT MAX(d.timestamp_ms)
                        FROM album_databases ad
                        JOIN database d ON ad.hash = d.hash
                        WHERE ad.album_id = OLD.album_id
                    ),
                    cover = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_databases WHERE album_id = OLD.album_id AND hash = cover) THEN cover
                        ELSE (
                            SELECT d.hash
                            FROM album_databases ad
                            JOIN database d ON ad.hash = d.hash
                            WHERE ad.album_id = OLD.album_id
                            ORDER BY d.timestamp_ms ASC
                            LIMIT 1
                        )
                    END,
                    thumbhash = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_databases WHERE album_id = OLD.album_id AND hash = cover) THEN thumbhash
                        ELSE (
                            SELECT d.thumbhash
                            FROM album_databases ad
                            JOIN database d ON ad.hash = d.hash
                            WHERE ad.album_id = OLD.album_id
                            ORDER BY d.timestamp_ms ASC
                            LIMIT 1
                        )
                    END,
                    last_modified_time = (strftime('%s', 'now') * 1000)
                WHERE id = OLD.album_id;
            END;
        "##;
        conn.execute_batch(sql)?;
        Ok(())
    }
}
