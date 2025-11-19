use crate::public::structure::{
    album::{Album, Share},
    database_struct::{database::definition::Database, file_modify::FileModify},
    expression::Expression,
    tag_info::TagInfo,
};
use arrayvec::ArrayString;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, ToSql, params};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

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
             PRAGMA synchronous = NORMAL;
             PRAGMA recursive_triggers = ON;",
        )
        .expect("Failed to set PRAGMA");

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL CHECK (kind IN ('image', 'video', 'album')),
                title TEXT,
                created_time INTEGER NOT NULL,
                last_modified_time INTEGER,
                pending BOOLEAN NOT NULL DEFAULT 0,
                width INTEGER NOT NULL DEFAULT 0,
                height INTEGER NOT NULL DEFAULT 0,
                start_time INTEGER,
                end_time INTEGER,
                size INTEGER,
                ext TEXT,
                ext_type TEXT,
                timestamp INTEGER,
                thumbhash BLOB,
                phash BLOB,
                exif TEXT,
                alias TEXT
            )",
            [],
        )
        .expect("Failed to create nodes table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS album_meta (
                album_id TEXT PRIMARY KEY,
                cover_id TEXT,
                user_defined_metadata TEXT NOT NULL DEFAULT '{}',
                share_list TEXT NOT NULL DEFAULT '{}',
                item_count INTEGER NOT NULL DEFAULT 0,
                item_size INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (album_id) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (cover_id) REFERENCES nodes(id)
            )",
            [],
        )
        .expect("Failed to create album_meta table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                timestamp INTEGER,
                idx INTEGER,
                node_id TEXT,
                PRIMARY KEY (timestamp, idx),
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            )",
            [],
        )
        .expect("Failed to create snapshots table");

        // Phase 5: Normalization
        conn.execute(
            "CREATE TABLE IF NOT EXISTS node_tags (
                node_id TEXT,
                tag TEXT,
                PRIMARY KEY (node_id, tag),
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            )",
            [],
        )
        .expect("Failed to create node_tags table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS album_items (
                album_id TEXT,
                item_id TEXT,
                PRIMARY KEY (album_id, item_id),
                FOREIGN KEY (album_id) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (item_id) REFERENCES nodes(id) ON DELETE CASCADE
            )",
            [],
        )
        .expect("Failed to create album_items table");

        // Create triggers for automatic maintenance
        conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS trg_album_items_ai
            AFTER INSERT ON album_items
            BEGIN
                UPDATE album_meta
                SET
                    item_count = item_count + 1,
                    item_size  = item_size + COALESCE(
                        (SELECT size FROM nodes WHERE id = NEW.item_id),
                        0
                    )
                WHERE album_id = NEW.album_id;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_album_items_ad
            AFTER DELETE ON album_items
            BEGIN
                UPDATE album_meta
                SET
                    item_count = item_count - 1,
                    item_size  = item_size - COALESCE(
                        (SELECT size FROM nodes WHERE id = OLD.item_id),
                        0
                    )
                WHERE album_id = OLD.album_id;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_album_items_au
            AFTER UPDATE OF item_id ON album_items
            BEGIN
                UPDATE album_meta
                SET item_size = item_size - COALESCE(
                    (SELECT size FROM nodes WHERE id = OLD.item_id),
                    0
                )
                WHERE album_id = NEW.album_id;

                UPDATE album_meta
                SET item_size = item_size + COALESCE(
                    (SELECT size FROM nodes WHERE id = NEW.item_id),
                    0
                )
                WHERE album_id = NEW.album_id;
            END;

            CREATE TRIGGER IF NOT EXISTS trg_nodes_size_au
            AFTER UPDATE OF size ON nodes
            WHEN NEW.size != OLD.size
            BEGIN
                UPDATE album_meta
                SET item_size = item_size + (NEW.size - OLD.size)
                WHERE album_id IN (
                    SELECT album_id
                    FROM album_items
                    WHERE item_id = NEW.id
                );
            END;
            "
        )
        .expect("Failed to create triggers");

        // Clear snapshots on startup
        conn.execute("DELETE FROM snapshots", [])
            .expect("Failed to clear snapshots");

        Self { pool }
    }

    pub fn get_database(&self, hash: &str) -> rusqlite::Result<Option<Database>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT id, size, width, height, ext, ext_type, pending, thumbhash, phash, exif, alias FROM nodes WHERE id = ? AND kind IN ('image', 'video')")?;

        let result = stmt
            .query_row(params![hash], |row| {
                let id: String = row.get(0)?;
                let size: u64 = row.get(1)?;
                let width: u32 = row.get(2)?;
                let height: u32 = row.get(3)?;
                let ext: String = row.get(4)?;
                let ext_type: String = row.get(5)?;
                let pending: bool = row.get(6)?;
                let thumbhash: Vec<u8> = row.get(7)?;
                let phash: Vec<u8> = row.get(8)?;
                let exif_json: String = row.get(9)?;
                let alias_json: String = row.get(10)?;

                let exif_vec: BTreeMap<String, String> =
                    serde_json::from_str(&exif_json).unwrap_or_default();
                let alias: Vec<FileModify> = serde_json::from_str(&alias_json).unwrap_or_default();

                Ok(Database {
                    hash: ArrayString::from(&id).unwrap_or_default(),
                    size,
                    width,
                    height,
                    thumbhash,
                    phash,
                    ext,
                    exif_vec,
                    tag: HashSet::new(),   // Will fill later
                    album: HashSet::new(), // Will fill later
                    alias,
                    ext_type,
                    pending,
                })
            })
            .optional()?;

        if let Some(mut database) = result {
            // Fetch tags
            let mut stmt_tags = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?")?;
            let tags_iter = stmt_tags.query_map(params![hash], |row| row.get(0))?;
            for tag in tags_iter {
                database.tag.insert(tag?);
            }

            // Fetch albums
            let mut stmt_albums =
                conn.prepare("SELECT album_id FROM album_items WHERE item_id = ?")?;
            let albums_iter = stmt_albums.query_map(params![hash], |row| row.get(0))?;
            for album_id in albums_iter {
                let aid: String = album_id?;
                database
                    .album
                    .insert(ArrayString::from(&aid).unwrap_or_default());
            }

            Ok(Some(database))
        } else {
            Ok(None)
        }
    }

    pub fn get_album(&self, id: &str) -> rusqlite::Result<Option<Album>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT n.id, n.title, n.created_time, n.pending, n.width, n.height, n.start_time, n.end_time, n.last_modified_time, am.cover_id, n.thumbhash, am.user_defined_metadata, am.share_list, am.item_count, am.item_size FROM nodes n LEFT JOIN album_meta am ON n.id = am.album_id WHERE n.id = ? AND n.kind = 'album'")?;

        let result = stmt
            .query_row(params![id], |row| {
                let id: String = row.get(0)?;
                let title: Option<String> = row.get(1)?;
                let created_time: i64 = row.get(2)?;
                let pending: bool = row.get(3)?;
                let width: u32 = row.get(4)?;
                let height: u32 = row.get(5)?;
                let start_time: Option<i64> = row.get(6)?;
                let end_time: Option<i64> = row.get(7)?;
                let last_modified_time: i64 = row.get(8)?;
                let cover_id: Option<String> = row.get(9)?;
                let thumbhash: Option<Vec<u8>> = row.get(10)?;
                let user_meta_json: String = row.get(11)?;
                let share_list_json: String = row.get(12)?;
                let item_count: usize = row.get(13)?;
                let item_size: i64 = row.get(14)?;

                let user_defined_metadata: HashMap<String, Vec<String>> =
                    serde_json::from_str(&user_meta_json).unwrap_or_default();
                let share_list: HashMap<ArrayString<64>, Share> =
                    serde_json::from_str(&share_list_json).unwrap_or_default();

                Ok(Album {
                    id: ArrayString::from(&id).unwrap_or_default(),
                    title,
                    created_time: created_time as u128,
                    start_time: start_time.map(|t| t as u128),
                    end_time: end_time.map(|t| t as u128),
                    last_modified_time: last_modified_time as u128,
                    cover: cover_id.map(|c| ArrayString::from(&c).unwrap_or_default()),
                    thumbhash,
                    user_defined_metadata,
                    share_list,
                    tag: HashSet::new(), // Will fill later
                    width,
                    height,
                    item_count,
                    item_size: item_size as u64,
                    pending,
                })
            })
            .optional()?;

        if let Some(mut album) = result {
            // Calculate stats on-read
            let (count, size, start, end, cover_db) = self.get_album_stats(id)?;

            album.item_count = count;
            album.item_size = size;
            album.start_time = start;
            album.end_time = end;

            // Validate cover
            let current_cover_valid = if let Some(cover_id) = &album.cover {
                self.is_object_in_album(cover_id, id).unwrap_or(false)
            } else {
                false
            };

            if !current_cover_valid {
                if let Some(db) = cover_db {
                    album.cover = Some(db.hash);
                    album.thumbhash = Some(db.thumbhash);
                    album.width = db.width;
                    album.height = db.height;
                } else {
                    album.cover = None;
                    album.thumbhash = None;
                    album.width = 0;
                    album.height = 0;
                }
            }

            // Fetch tags
            let mut stmt_tags = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?")?;
            let tags_iter = stmt_tags.query_map(params![id], |row| row.get(0))?;
            for tag in tags_iter {
                album.tag.insert(tag?);
            }

            Ok(Some(album))
        } else {
            Ok(None)
        }
    }

    pub fn get_objects_count(&self) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM objects")?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn get_albums_count(&self) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM albums")?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn get_all_tags(&self) -> rusqlite::Result<Vec<TagInfo>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            "SELECT tag, COUNT(*) 
             FROM node_tags 
             GROUP BY tag",
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

    pub fn get_all_albums(&self) -> rusqlite::Result<Vec<Album>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM nodes WHERE kind = 'album'")?;
        let ids_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut albums = Vec::new();
        for id in ids_iter {
            if let Ok(Some(album)) = self.get_album(&id?) {
                albums.push(album);
            }
        }
        Ok(albums)
    }

    pub fn get_album_stats(
        &self,
        album_id: &str,
    ) -> rusqlite::Result<(usize, u64, Option<u128>, Option<u128>, Option<Database>)> {
        let conn = self.pool.get().unwrap();

        // Aggregates
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), IFNULL(SUM(nodes.size), 0), MIN(nodes.timestamp), MAX(nodes.timestamp) 
             FROM album_items 
             JOIN nodes ON album_items.item_id = nodes.id 
             WHERE album_items.album_id = ?"
        )?;

        let (count, size, start, end) = stmt.query_row(params![album_id], |row| {
            Ok((
                row.get::<_, usize>(0)?,
                row.get::<_, i64>(1)? as u64,
                row.get::<_, Option<i64>>(2)?.map(|t| t as u128),
                row.get::<_, Option<i64>>(3)?.map(|t| t as u128),
            ))
        })?;

        if count == 0 {
            return Ok((0, 0, None, None, None));
        }

        // Cover (First item by timestamp)
        let mut stmt = conn.prepare(
            "SELECT nodes.id 
             FROM album_items 
             JOIN nodes ON album_items.item_id = nodes.id 
             WHERE album_items.album_id = ? 
             ORDER BY nodes.timestamp ASC 
             LIMIT 1",
        )?;

        let cover_id: Option<String> = stmt
            .query_row(params![album_id], |row| row.get(0))
            .optional()?;

        let cover_db = if let Some(id) = cover_id {
            self.get_database(&id)?
        } else {
            None
        };

        Ok((count, size, start, end, cover_db))
    }

    pub fn is_object_in_album(&self, object_id: &str, album_id: &str) -> rusqlite::Result<bool> {
        let conn = self.pool.get().unwrap();
        let mut stmt =
            conn.prepare("SELECT 1 FROM album_items WHERE album_id = ? AND item_id = ?")?;
        Ok(stmt.exists(params![album_id, object_id])?)
    }

    pub fn _get_objects_in_album(&self, album_id: &str) -> rusqlite::Result<Vec<String>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT item_id FROM album_items WHERE album_id = ?")?;
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
        let mut stmt =
            conn.prepare("SELECT node_id FROM snapshots WHERE timestamp = ? AND idx = ?")?;
        stmt.query_row(params![timestamp as i64, idx], |row| row.get(0))
    }

    pub fn get_snapshot_width_height(
        &self,
        timestamp: u128,
        idx: usize,
    ) -> rusqlite::Result<(u32, u32)> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare(
            "SELECT nodes.width, nodes.height
             FROM snapshots s
             JOIN nodes ON s.node_id = nodes.id
             WHERE s.timestamp = ? AND s.idx = ?",
        )?;
        
        stmt.query_row(params![timestamp as i64, idx], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
    }    pub fn get_all_objects(&self) -> rusqlite::Result<Vec<Database>> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT id, size, width, height, ext, ext_type, pending, thumbhash, phash, exif, alias FROM nodes WHERE kind IN ('image', 'video')")?;
        let iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let size: u64 = row.get(1)?;
            let width: u32 = row.get(2)?;
            let height: u32 = row.get(3)?;
            let ext: String = row.get(4)?;
            let ext_type: String = row.get(5)?;
            let pending: bool = row.get(6)?;
            let thumbhash: Vec<u8> = row.get(7)?;
            let phash: Vec<u8> = row.get(8)?;
            let exif_json: String = row.get(9)?;
            let alias_json: String = row.get(10)?;

            let exif_vec: BTreeMap<String, String> =
                serde_json::from_str(&exif_json).unwrap_or_default();
            let alias: Vec<FileModify> = serde_json::from_str(&alias_json).unwrap_or_default();

            let mut database = Database {
                hash: ArrayString::from(&id).unwrap_or_default(),
                size,
                width,
                height,
                thumbhash,
                phash,
                ext,
                exif_vec,
                tag: HashSet::new(),   // Will fill later
                album: HashSet::new(), // Will fill later
                alias,
                ext_type,
                pending,
            };

            // Fetch tags
            let mut stmt_tags = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?")?;
            let tags_iter = stmt_tags.query_map(params![&id], |row| row.get(0))?;
            for tag in tags_iter {
                database.tag.insert(tag?);
            }

            // Fetch albums
            let mut stmt_albums =
                conn.prepare("SELECT album_id FROM album_items WHERE item_id = ?")?;
            let albums_iter = stmt_albums.query_map(params![&id], |row| row.get(0))?;
            for album_id in albums_iter {
                let aid: String = album_id?;
                database
                    .album
                    .insert(ArrayString::from(&aid).unwrap_or_default());
            }

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
            "SELECT s.idx, nodes.timestamp
             FROM snapshots s
             JOIN nodes ON s.node_id = nodes.id
             WHERE s.timestamp = ?
             ORDER BY s.idx ASC",
        )?;
        
        let iter = stmt.query_map(params![timestamp as i64], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        let mut dates = Vec::new();
        for date in iter {
            dates.push(date?);
        }
        Ok(dates)
    }    pub fn generate_snapshot(
        &self,
        timestamp: u128,
        expression: &Option<Expression>,
        hide_metadata: bool,
        shared_album_id: Option<&str>,
    ) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let (where_clause, params) = if let Some(expr) = expression {
            expr.to_sql(hide_metadata, shared_album_id)
        } else {
            ("1=1".to_string(), vec![])
        };

        // Note: timestamp is cast to i64 for SQLite INTEGER compatibility
        let sql = format!(
            "INSERT INTO snapshots (timestamp, idx, node_id)
             SELECT ?, ROW_NUMBER() OVER (ORDER BY timestamp DESC) - 1, id
             FROM nodes
             WHERE kind IN ('image', 'video') AND {}",
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

    pub fn get_snapshot_index(
        &self,
        timestamp: u128,
        hash: &str,
    ) -> rusqlite::Result<Option<usize>> {
        let conn = self.pool.get().unwrap();
        let mut stmt =
            conn.prepare("SELECT idx FROM snapshots WHERE timestamp = ? AND node_id = ?")?;
        stmt.query_row(params![timestamp as i64, hash], |row| row.get(0))
            .optional()
    }

    pub fn delete_expired_snapshots(&self, timestamp_threshold: u128) -> rusqlite::Result<usize> {
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare("DELETE FROM snapshots WHERE timestamp < ?")?;
        stmt.execute(params![timestamp_threshold as i64])
    }

    pub fn delete_expired_pending_data(
        &self,
        timestamp_threshold: u128,
    ) -> rusqlite::Result<(usize, usize)> {
        let conn = self.pool.get().unwrap();

        let mut stmt_obj =
            conn.prepare("DELETE FROM nodes WHERE pending = 1 AND timestamp < ? AND kind IN ('image', 'video')")?;
        let obj_count = stmt_obj.execute(params![timestamp_threshold as i64])?;

        let mut stmt_album =
            conn.prepare("DELETE FROM nodes WHERE pending = 1 AND created_time < ? AND kind = 'album'")?;
        let album_count = stmt_album.execute(params![timestamp_threshold as i64])?;

        Ok((obj_count, album_count))
    }
}

pub static SQLITE: LazyLock<Sqlite> = LazyLock::new(|| Sqlite::new());
