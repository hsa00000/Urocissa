use rusqlite::Connection;

pub struct AlbumDatabasesTable;

impl AlbumDatabasesTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r##"
            CREATE TABLE IF NOT EXISTS album_databases (
                album_id TEXT NOT NULL,
                hash     TEXT NOT NULL,
                PRIMARY KEY (album_id, hash),
                FOREIGN KEY (album_id) REFERENCES object(id) ON DELETE CASCADE,
                FOREIGN KEY (hash)     REFERENCES object(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_album_databases_hash
            ON album_databases(hash);

            -- Trigger: Insert
            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_insert AFTER INSERT ON album_databases
            BEGIN
                UPDATE meta_album SET
                    item_count = (SELECT COUNT(*) FROM album_databases WHERE album_id = NEW.album_id),
                    item_size = (
                        SELECT COALESCE(SUM(COALESCE(mi.size, mv.size, 0)), 0)
                        FROM album_databases ad
                        JOIN object o ON ad.hash = o.id
                        LEFT JOIN meta_image mi ON o.id = mi.id
                        LEFT JOIN meta_video mv ON o.id = mv.id
                        WHERE ad.album_id = NEW.album_id
                    ),
                    start_time = (
                        SELECT MIN(o.created_time)
                        FROM album_databases ad
                        JOIN object o ON ad.hash = o.id
                        WHERE ad.album_id = NEW.album_id
                    ),
                    end_time = (
                        SELECT MAX(o.created_time)
                        FROM album_databases ad
                        JOIN object o ON ad.hash = o.id
                        WHERE ad.album_id = NEW.album_id
                    ),
                    cover = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_databases WHERE album_id = NEW.album_id AND hash = cover) THEN cover
                        ELSE (
                            SELECT o.id
                            FROM album_databases ad
                            JOIN object o ON ad.hash = o.id
                            WHERE ad.album_id = NEW.album_id
                            ORDER BY o.created_time ASC
                            LIMIT 1
                        )
                    END,
                    last_modified_time = (strftime('%s', 'now') * 1000)
                WHERE id = NEW.album_id;
            END;

            -- Trigger: Delete
            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_delete AFTER DELETE ON album_databases
            BEGIN
                UPDATE meta_album SET
                    item_count = (SELECT COUNT(*) FROM album_databases WHERE album_id = OLD.album_id),
                    item_size = (
                        SELECT COALESCE(SUM(COALESCE(mi.size, mv.size, 0)), 0)
                        FROM album_databases ad
                        JOIN object o ON ad.hash = o.id
                        LEFT JOIN meta_image mi ON o.id = mi.id
                        LEFT JOIN meta_video mv ON o.id = mv.id
                        WHERE ad.album_id = OLD.album_id
                    ),
                    start_time = (
                        SELECT MIN(o.created_time)
                        FROM album_databases ad
                        JOIN object o ON ad.hash = o.id
                        WHERE ad.album_id = OLD.album_id
                    ),
                    end_time = (
                        SELECT MAX(o.created_time)
                        FROM album_databases ad
                        JOIN object o ON ad.hash = o.id
                        WHERE ad.album_id = OLD.album_id
                    ),
                    cover = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_databases WHERE album_id = OLD.album_id AND hash = cover) THEN cover
                        ELSE (
                            SELECT o.id
                            FROM album_databases ad
                            JOIN object o ON ad.hash = o.id
                            WHERE ad.album_id = OLD.album_id
                            ORDER BY o.created_time ASC
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
