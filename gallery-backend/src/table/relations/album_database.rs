use rusqlite::Connection;

pub struct AlbumDatabasesTable;

impl AlbumDatabasesTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r##"
            CREATE TABLE IF NOT EXISTS album_database (
                album_id TEXT NOT NULL,
                hash     TEXT NOT NULL,
                PRIMARY KEY (album_id, hash),
                FOREIGN KEY (album_id) REFERENCES object(id) ON DELETE CASCADE,
                FOREIGN KEY (hash)     REFERENCES object(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_album_databases_hash
            ON album_database(hash);

            -- Trigger: Insert
            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_insert AFTER INSERT ON album_database
            BEGIN
                UPDATE meta_album SET
                    item_count = (SELECT COUNT(*) FROM album_database WHERE album_id = NEW.album_id),
                    item_size = (
                        SELECT COALESCE(SUM(COALESCE(meta_image.size, meta_video.size, 0)), 0)
                        FROM album_database
                        JOIN object ON album_database.hash = object.id
                        LEFT JOIN meta_image ON object.id = meta_image.id
                        LEFT JOIN meta_video ON object.id = meta_video.id
                        WHERE album_database.album_id = NEW.album_id
                    ),
                    start_time = (
                        SELECT MIN(object.created_time)
                        FROM album_database
                        JOIN object ON album_database.hash = object.id
                        WHERE album_database.album_id = NEW.album_id
                    ),
                    end_time = (
                        SELECT MAX(object.created_time)
                        FROM album_database
                        JOIN object ON album_database.hash = object.id
                        WHERE album_database.album_id = NEW.album_id
                    ),
                    cover = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_database WHERE album_id = NEW.album_id AND hash = cover) THEN cover
                        ELSE (
                            SELECT object.id
                            FROM album_database
                            JOIN object ON album_database.hash = object.id
                            WHERE album_database.album_id = NEW.album_id
                            ORDER BY object.created_time ASC
                            LIMIT 1
                        )
                    END,
                    last_modified_time = (strftime('%s', 'now') * 1000)
                WHERE id = NEW.album_id;
            END;

            -- Trigger: Delete
            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_delete AFTER DELETE ON album_database
            BEGIN
                UPDATE meta_album SET
                    item_count = (SELECT COUNT(*) FROM album_database WHERE album_id = OLD.album_id),
                    item_size = (
                        SELECT COALESCE(SUM(COALESCE(meta_image.size, meta_video.size, 0)), 0)
                        FROM album_database
                        JOIN object ON album_database.hash = object.id
                        LEFT JOIN meta_image ON object.id = meta_image.id
                        LEFT JOIN meta_video ON object.id = meta_video.id
                        WHERE album_database.album_id = OLD.album_id
                    ),
                    start_time = (
                        SELECT MIN(object.created_time)
                        FROM album_database
                        JOIN object ON album_database.hash = object.id
                        WHERE album_database.album_id = OLD.album_id
                    ),
                    end_time = (
                        SELECT MAX(object.created_time)
                        FROM album_database
                        JOIN object ON album_database.hash = object.id
                        WHERE album_database.album_id = OLD.album_id
                    ),
                    cover = CASE
                        WHEN cover IS NOT NULL AND EXISTS (SELECT 1 FROM album_database WHERE album_id = OLD.album_id AND hash = cover) THEN cover
                        ELSE (
                            SELECT object.id
                            FROM album_database
                            JOIN object ON album_database.hash = object.id
                            WHERE album_database.album_id = OLD.album_id
                            ORDER BY object.created_time ASC
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
