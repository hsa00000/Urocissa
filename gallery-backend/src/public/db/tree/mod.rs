pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Album;
use crate::public::structure::database_struct::database::definition::Database;
use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use std::sync::{Arc, LazyLock, RwLock, atomic::AtomicU64};

pub struct Tree {
    pub in_disk: Pool<SqliteConnectionManager>,
    pub in_memory: &'static Arc<RwLock<Vec<DatabaseTimestamp>>>,
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
            "SELECT * FROM database_with_tags WHERE hash = ?",
            [id],
            Database::from_row,
        ) {
            Ok(AbstractData::Database(database))
        } else if let Ok(album) =
            conn.query_row("SELECT * FROM album WHERE id = ?", [id], Album::from_row)
        {
            Ok(AbstractData::Album(album))
        } else {
            Err(anyhow::anyhow!("No data found for id: {}", id))
        }
    }

    pub fn load_all_databases_from_db(&self) -> Result<Vec<Database>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database_with_tags")?;
        let rows = stmt.query_map([], Database::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)
    }

    pub fn load_database_from_hash(&self, hash: &str) -> Result<Database> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM database_with_tags WHERE hash = ?")?;
        stmt.query_row([hash], Database::from_row)
            .map_err(anyhow::Error::from)
    }
}
