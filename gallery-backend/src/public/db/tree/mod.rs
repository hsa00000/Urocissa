pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::public::structure::abstract_data::AbstractData;
use crate::table::album::AlbumSchema;
use crate::table::database::DatabaseSchema;
use crate::tasks::actor::index::IndexTask;
use std::path::PathBuf;
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
        if let Ok(database) = conn.query_row(
            "SELECT * FROM database WHERE hash = ?",
            [id],
            DatabaseSchema::from_row,
        ) {
            Ok(AbstractData::DatabaseSchema(database))
        } else if let Ok(album) =
            conn.query_row("SELECT * FROM album WHERE id = ?", [id], AlbumSchema::from_row)
        {
            Ok(AbstractData::Album(album))
        } else {
            Err(anyhow::anyhow!("No data found for id: {}", id))
        }
    }

    pub fn load_all_databases_from_db(&self) -> Result<Vec<DatabaseSchema>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database")?;
        let rows = stmt.query_map([], DatabaseSchema::from_row)?;
        let mut databases = Vec::new();
        for row in rows {
            databases.push(row?);
        }
        Ok(databases)
    }

    pub fn load_database_from_hash(&self, hash: &str) -> Result<DatabaseSchema> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database WHERE hash = ?")?;
        stmt.query_row([hash], DatabaseSchema::from_row)
            .map_err(anyhow::Error::from)
    }

    pub fn load_index_task_from_hash(&self, hash: &str) -> Result<IndexTask> {
        let database = self.load_database_from_hash(hash)?;
        let source_path = PathBuf::from(format!(
            "./object/imported/{}/{}.{}",
            &hash[0..2],
            hash,
            database.ext
        ));
        Ok(IndexTask::new(source_path, database))
    }
}
