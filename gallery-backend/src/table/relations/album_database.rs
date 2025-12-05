use crate::table::object::Object;
use rusqlite::Connection;
use sea_query::{ColumnDef, ForeignKey, Iden, Index, SqliteQueryBuilder, Table};

#[derive(Iden)]
pub enum AlbumDatabase {
    Table, // "album_database"
    AlbumId,
    Hash,
}

pub struct AlbumDatabasesTable;

impl AlbumDatabasesTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        // 1. 使用 SeaQuery 建立 Table
        let table_sql = Table::create()
            .table(AlbumDatabase::Table)
            .if_not_exists()
            .col(ColumnDef::new(AlbumDatabase::AlbumId).text().not_null())
            .col(ColumnDef::new(AlbumDatabase::Hash).text().not_null())
            .primary_key(
                Index::create()
                    .col(AlbumDatabase::AlbumId)
                    .col(AlbumDatabase::Hash),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AlbumDatabase::Table, AlbumDatabase::AlbumId)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AlbumDatabase::Table, AlbumDatabase::Hash)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);
        conn.execute(&table_sql, [])?;

        // 2. 使用 SeaQuery 建立 Index
        let idx_sql = Index::create()
            .if_not_exists()
            .name("idx_album_databases_hash")
            .table(AlbumDatabase::Table)
            .col(AlbumDatabase::Hash)
            .to_string(SqliteQueryBuilder);
        conn.execute(&idx_sql, [])?;

        // 3. 保留 Raw SQL Trigger
        // 注意：Trigger 名稱、欄位名稱必須與上方 Enum 定義的字串一致 (SeaQuery 預設 Snake Case)
        let trigger_sql = r##"
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
        conn.execute_batch(trigger_sql)?;
        Ok(())
    }
}
