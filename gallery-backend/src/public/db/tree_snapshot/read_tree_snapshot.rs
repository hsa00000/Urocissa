use super::TreeSnapshot;
use crate::public::structure::reduced_data::ReducedData;
use crate::public::db::sqlite::SQLITE;
use anyhow::Result;
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        Ok(MyCow::Sqlite(*timestamp))
    }
}

#[derive(Debug)]
pub enum MyCow {
    DashMap(Ref<'static, u128, Vec<ReducedData>>),
    Sqlite(u128),
}

impl MyCow {
    pub fn len(&self) -> usize {
        match self {
            MyCow::DashMap(data) => data.value().len(),
            MyCow::Sqlite(timestamp) => SQLITE.get_snapshot_len(*timestamp).unwrap_or(0),
        }
    }

    pub fn get_width_height(&self, index: usize) -> Result<(u32, u32)> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok((data.width, data.height))
            }
            MyCow::Sqlite(timestamp) => {
                Ok(SQLITE.get_snapshot_width_height(*timestamp, index)?)
            }
        }
    }

    pub fn get_hash(&self, index: usize) -> Result<ArrayString<64>> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok(data.hash)
            }
            MyCow::Sqlite(timestamp) => {
                let hash_str = SQLITE.get_snapshot_hash(*timestamp, index)?;
                Ok(ArrayString::from(&hash_str).unwrap_or_default())
            }
        }
    }
}
