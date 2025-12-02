pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::{HashMap, HashSet};

use crate::public::structure::abstract_data::{AbstractData, Database};
use crate::table::album::AlbumSchema;
use crate::table::database::DatabaseSchema;
use std::sync::{Arc, LazyLock, RwLock, atomic::AtomicU64};

pub struct Tree {
    pub in_disk: Pool<SqliteConnectionManager>,
    pub in_memory: &'static Arc<RwLock<Vec<AbstractData>>>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree::new());

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl Tree {
    pub fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        let conn = self.in_disk.get().context("Failed to get DB connection")?;
        Ok(conn)
    }
    pub fn load_from_db(&self, id: &str) -> Result<AbstractData> {
        let conn = self.get_connection()?;
        if let Ok(schema) = conn.query_row(
            "SELECT * FROM database WHERE hash = ?",
            [id],
            DatabaseSchema::from_row,
        ) {
            // 讀取相簿關聯
            let mut stmt = conn.prepare("SELECT album_id FROM album_databases WHERE hash = ?")?;
            let albums = stmt.query_map([id], |row| row.get::<_, String>(0))?;
            let mut album_set = HashSet::new();
            for album_id in albums {
                if let Ok(as_str) = ArrayString::from(&album_id?) {
                    album_set.insert(as_str);
                }
            }
            Ok(AbstractData::Database(Database {
                schema,
                album: album_set,
            }))
        } else if let Ok(album) = conn.query_row(
            "SELECT * FROM album WHERE id = ?",
            [id],
            AlbumSchema::from_row,
        ) {
            Ok(AbstractData::Album(album))
        } else {
            Err(anyhow::anyhow!("No data found for id: {}", id))
        }
    }

    // 修改回傳型別為 Vec<Database>，因為單純的 Schema 在這裡可能不夠用
    pub fn load_all_databases_from_db(&self) -> Result<Vec<Database>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database")?;
        let rows = stmt.query_map([], DatabaseSchema::from_row)?;

        let mut databases_map: HashMap<String, Database> = HashMap::new();

        for row in rows {
            let schema = row?;
            let hash = schema.hash.as_str().to_string();
            databases_map.insert(
                hash,
                Database {
                    schema,
                    album: HashSet::new(),
                },
            );
        }

        // 批量讀取 album_databases
        let mut stmt_albums = conn.prepare("SELECT hash, album_id FROM album_databases")?;
        let album_rows = stmt_albums.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in album_rows {
            let (hash, album_id) = row?;
            if let Some(db) = databases_map.get_mut(&hash) {
                if let Ok(as_str) = ArrayString::from(&album_id) {
                    db.album.insert(as_str);
                }
            }
        }

        Ok(databases_map.into_values().collect())
    }

    pub fn load_database_from_hash(&self, hash: &str) -> Result<Database> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database WHERE hash = ?")?;
        let schema = stmt.query_row([hash], DatabaseSchema::from_row)?;

        // 讀取相簿關聯
        let mut stmt_albums =
            conn.prepare("SELECT album_id FROM album_databases WHERE hash = ?")?;
        let albums = stmt_albums.query_map([hash], |row| row.get::<_, String>(0))?;
        let mut album_set = HashSet::new();
        for album_id in albums {
            if let Ok(as_str) = ArrayString::from(&album_id?) {
                album_set.insert(as_str);
            }
        }

        Ok(Database {
            schema,
            album: album_set,
        })
    }
}
