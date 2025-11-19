use std::sync::LazyLock;
use rusqlite::{OptionalExtension, params, ToSql};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use crate::public::structure::{
    album::Album,
    database_struct::database::definition::Database,
    expression::Expression,
    tag_info::TagInfo,
};

pub struct Sqlite {
    pub pool: Pool<SqliteConnectionManager>,
}

impl Sqlite {
    pub fn new() -> Self {
        let path = "./db/sqlite.db";
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager).expect("Failed to create pool");
        
        let conn = pool.get().expect("Failed to get connection");

        // Enable WAL mode for better concurrency
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .expect("Failed to set PRAGMA");

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                data BLOB,
                size INTEGER,
                width INTEGER,
                height INTEGER,
                ext TEXT,
                ext_type TEXT,
                pending BOOLEAN,
                timestamp INTEGER
            )",
            [],
        )
        .expect("Failed to create objects table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS albums (
                id TEXT PRIMARY KEY,
                data BLOB,
                title TEXT,
                created_time INTEGER,
                pending BOOLEAN,
                width INTEGER,
                height INTEGER
            )",
            [],
        )
        .expect("Failed to create albums table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                timestamp INTEGER,
                idx INTEGER,
                hash TEXT,
                PRIMARY KEY (timestamp, idx)
            )",
            [],
        )
        .expect("Failed to create snapshots table");

        Self {
            pool,
        }
    }

    pub fn get_database(&self, hash: &str) -> rusqlite::Result<Option<Database>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM objects WHERE id = ?")?;
        let data: Option<Vec<u8>> = stmt
            .query_row(params![hash], |row| row.get(0))
            .optional()?;

        match data {
            Some(bytes) => {
                let database: Database = serde_json::from_slice(&bytes).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(e),
                    )
                })?;
                Ok(Some(database))
            }
            None => Ok(None),
        }
    }

    pub fn get_album(&self, id: &str) -> rusqlite::Result<Option<Album>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM albums WHERE id = ?")?;
        let data: Option<Vec<u8>> = stmt
            .query_row(params![id], |row| row.get(0))
            .optional()?;

        match data {
            Some(bytes) => {
                let album: Album = serde_json::from_slice(&bytes).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(e),
                    )
                })?;
                Ok(Some(album))
            }
            None => Ok(None),
        }
    }

    pub fn get_all_albums(&self) -> rusqlite::Result<Vec<Album>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM albums")?;
        let album_iter = stmt.query_map([], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            let album: Album = serde_json::from_slice(&bytes).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                )
            })?;
            Ok(album)
        })?;

        let mut albums = Vec::new();
        for album in album_iter {
            albums.push(album?);
        }
        Ok(albums)
    }

    pub fn get_all_tags(&self) -> rusqlite::Result<Vec<TagInfo>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            "SELECT value, COUNT(*) 
             FROM objects, json_each(objects.data, '$.tag') 
             GROUP BY value"
        )?;
        
        let iter = stmt.query_map([], |row| {
            Ok(TagInfo {
                tag: row.get(0)?,
                number: row.get(1)?,
            })
        })?;

        let mut tags = Vec::new();
        for tag in iter {
            tags.push(tag?);
        }
        Ok(tags)
    }

    pub fn get_album_stats(&self, album_id: &str) -> rusqlite::Result<(usize, u64, Option<u128>, Option<u128>, Option<Database>)> {
        let conn = self.pool.get().unwrap();
        
        // Aggregates
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), SUM(size), MIN(timestamp), MAX(timestamp) 
             FROM objects 
             WHERE EXISTS (SELECT 1 FROM json_each(objects.data, '$.album') WHERE value = ?)"
        )?;
        
        let (count, size, start, end) = stmt.query_row(params![album_id], |row| {
            Ok((
                row.get::<_, usize>(0)?,
                row.get::<_, Option<u64>>(1)?.unwrap_or(0),
                row.get::<_, Option<i64>>(2)?.map(|t| t as u128),
                row.get::<_, Option<i64>>(3)?.map(|t| t as u128),
            ))
        })?;

        if count == 0 {
             return Ok((0, 0, None, None, None));
        }

        // Cover (First item)
        let mut stmt = conn.prepare(
            "SELECT data 
             FROM objects 
             WHERE EXISTS (SELECT 1 FROM json_each(objects.data, '$.album') WHERE value = ?) 
             ORDER BY timestamp ASC 
             LIMIT 1"
        )?;
        
        let cover_data: Option<Vec<u8>> = stmt.query_row(params![album_id], |row| row.get(0)).optional()?;
        
        let cover_db = if let Some(bytes) = cover_data {
             Some(serde_json::from_slice(&bytes).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                )
            })?)
        } else {
            None
        };

        Ok((count, size, start, end, cover_db))
    }

    pub fn is_object_in_album(&self, object_id: &str, album_id: &str) -> rusqlite::Result<bool> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            "SELECT 1 FROM objects 
             WHERE id = ? AND EXISTS (SELECT 1 FROM json_each(objects.data, '$.album') WHERE value = ?)"
        )?;
        Ok(stmt.exists(params![object_id, album_id])?)
    }

    pub fn get_objects_in_album(&self, album_id: &str) -> rusqlite::Result<Vec<String>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id FROM objects 
             WHERE EXISTS (SELECT 1 FROM json_each(objects.data, '$.album') WHERE value = ?)
            ")?;
        let iter = stmt.query_map(params![album_id], |row| row.get(0))?;
        let mut ids = Vec::new();
        for id in iter {
            ids.push(id?);
        }
        Ok(ids)
    }

    pub fn get_snapshot_len(&self, timestamp: u128) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM snapshots WHERE timestamp = ?")?;
        stmt.query_row(params![timestamp as i64], |row| row.get(0))
    }

    pub fn get_snapshot_hash(&self, timestamp: u128, idx: usize) -> rusqlite::Result<String> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT hash FROM snapshots WHERE timestamp = ? AND idx = ?")?;
        stmt.query_row(params![timestamp as i64, idx], |row| row.get(0))
    }

    pub fn get_snapshot_width_height(&self, timestamp: u128, idx: usize) -> rusqlite::Result<(u32, u32)> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(o.width, a.width), COALESCE(o.height, a.height)
             FROM snapshots s
             LEFT JOIN objects o ON s.hash = o.id
             LEFT JOIN albums a ON s.hash = a.id
             WHERE s.timestamp = ? AND s.idx = ?"
        )?;
        
        stmt.query_row(params![timestamp as i64, idx], |row| Ok((row.get(0)?, row.get(1)?)))
    }

    pub fn get_all_objects(&self) -> rusqlite::Result<Vec<Database>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM objects")?;
        let iter = stmt.query_map([], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            let database: Database = serde_json::from_slice(&bytes).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                )
            })?;
            Ok(database)
        })?;

        let mut objects = Vec::new();
        for obj in iter {
            objects.push(obj?);
        }
        Ok(objects)
    }

    pub fn get_snapshot_dates(&self, timestamp: u128) -> rusqlite::Result<Vec<(usize, i64)>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.idx, COALESCE(o.timestamp, a.created_time)
             FROM snapshots s
             LEFT JOIN objects o ON s.hash = o.id
             LEFT JOIN albums a ON s.hash = a.id
             WHERE s.timestamp = ?
             ORDER BY s.idx ASC"
        )?;
        
        let iter = stmt.query_map(params![timestamp as i64], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        let mut dates = Vec::new();
        for date in iter {
            dates.push(date?);
        }
        Ok(dates)
    }

    pub fn generate_snapshot(&self, timestamp: u128, expression: &Option<Expression>, hide_metadata: bool, shared_album_id: Option<&str>) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let (where_clause, params) = if let Some(expr) = expression {
            expr.to_sql(hide_metadata, shared_album_id)
        } else {
            ("1=1".to_string(), vec![])
        };

        // Note: timestamp is cast to i64 for SQLite INTEGER compatibility
        let sql = format!(
            "INSERT INTO snapshots (timestamp, idx, hash)
             SELECT ?, ROW_NUMBER() OVER (ORDER BY timestamp DESC) - 1, id
             FROM objects
             WHERE {}",
            where_clause
        );

        let mut stmt = conn.prepare(&sql)?;
        
        // Combine timestamp param with expression params
        let timestamp_i64 = timestamp as i64;
        let mut sql_params: Vec<&dyn ToSql> = vec![&timestamp_i64];
        let params_refs: Vec<&dyn ToSql> = params.iter().map(|p| &**p as &dyn ToSql).collect();
        sql_params.extend(params_refs);

        let count = stmt.execute(sql_params.as_slice())?;
        Ok(count)
    }

    pub fn get_snapshot_index(&self, timestamp: u128, hash: &str) -> rusqlite::Result<Option<usize>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT idx FROM snapshots WHERE timestamp = ? AND hash = ?")?;
        stmt.query_row(params![timestamp as i64, hash], |row| row.get(0)).optional()
    }

    pub fn get_latest_snapshot_timestamp(&self) -> rusqlite::Result<Option<u128>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT MAX(timestamp) FROM snapshots")?;
        let timestamp: Option<i64> = stmt.query_row([], |row| row.get(0)).optional()?;
        Ok(timestamp.map(|t| t as u128))
    }

    pub fn delete_expired_snapshots(&self, timestamp_threshold: u128) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("DELETE FROM snapshots WHERE timestamp < ?")?;
        stmt.execute(params![timestamp_threshold as i64])
    }

    pub fn delete_expired_pending_data(&self, timestamp_threshold: u128) -> rusqlite::Result<(usize, usize)> {
        let conn = self.pool.get().unwrap();
        
        let mut stmt_obj = conn.prepare("DELETE FROM objects WHERE pending = 1 AND timestamp < ?")?;
        let obj_count = stmt_obj.execute(params![timestamp_threshold as i64])?;

        let mut stmt_album = conn.prepare("DELETE FROM albums WHERE pending = 1 AND created_time < ?")?;
        let album_count = stmt_album.execute(params![timestamp_threshold as i64])?;

        Ok((obj_count, album_count))
    }
}

pub static SQLITE: LazyLock<Sqlite> = LazyLock::new(|| Sqlite::new());
