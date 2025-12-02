use super::TreeSnapshot;
use crate::public::structure::reduced_data::ReducedData;
use anyhow::{Context, Result};
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        // 不需要開啟 transaction，只需要 pool 和 timestamp 即可
        Ok(MyCow::Sqlite(self.in_disk, timestamp.to_string()))
    }
}

#[derive(Debug)]
pub enum MyCow {
    DashMap(Ref<'static, u128, Vec<ReducedData>>),
    Sqlite(&'static Pool<SqliteConnectionManager>, String),
}

impl MyCow {
    pub fn len(&self) -> usize {
        match self {
            MyCow::DashMap(data) => data.value().len(),
            MyCow::Sqlite(pool, timestamp) => {
                let conn = pool.get().unwrap();
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM snapshots WHERE timestamp = ?",
                        [timestamp],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                count as usize
            }
        }
    }

    pub fn get_width_height(&self, index: usize) -> Result<(u32, u32)> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok((data.width, data.height))
            }
            MyCow::Sqlite(pool, timestamp) => {
                let conn = pool.get().context("Failed to get DB connection")?;
                let data: Vec<u8> = conn
                    .query_row(
                        "SELECT data FROM snapshots WHERE timestamp = ? AND row_index = ?",
                        rusqlite::params![timestamp, index],
                        |row| row.get(0),
                    )
                    .context(format!(
                        "Fail to find with and height in tree snapshots for index {}",
                        index
                    ))?;
                
                let reduced: ReducedData = bitcode::decode(&data)
                    .context("Failed to decode ReducedData")?;

                Ok((reduced.width, reduced.height))
            }
        }
    }

    pub fn get_hash(&self, index: usize) -> Result<ArrayString<64>> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok(data.hash)
            }
            MyCow::Sqlite(pool, timestamp) => {
                let conn = pool.get().context("Failed to get DB connection")?;
                let data: Vec<u8> = conn
                    .query_row(
                        "SELECT data FROM snapshots WHERE timestamp = ? AND row_index = ?",
                        rusqlite::params![timestamp, index],
                        |row| row.get(0),
                    )
                    .context(format!(
                        "Fail to find hash in tree snapshots for index {}",
                        index
                    ))?;
                
                let reduced: ReducedData = bitcode::decode(&data)
                    .context("Failed to decode ReducedData")?;

                Ok(reduced.hash)
            }
        }
    }
}
