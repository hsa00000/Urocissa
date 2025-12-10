use super::{TreeSnapshot, new::SNAPSHOTS_TABLE};
use crate::models::entity::reduced_data::ReducedData;
use anyhow::{Context, Result};
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;
use redb::{ReadTransaction, ReadableDatabase};

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }
        let txn = self.in_disk.begin_read()?;
        Ok(MyCow::Redb(txn, *timestamp))
    }
}

pub enum MyCow {
    DashMap(Ref<'static, u128, Vec<ReducedData>>),
    Redb(ReadTransaction, u128),
}

impl MyCow {
    pub fn len(&self) -> usize {
        match self {
            MyCow::DashMap(data) => data.value().len(),
            MyCow::Redb(txn, timestamp) => {
                let table = txn.open_table(SNAPSHOTS_TABLE).unwrap();
                let start = (*timestamp, 0);
                let end = (*timestamp, u64::MAX);
                table.range(start..=end).unwrap().count()
            }
        }
    }

    pub fn get_width_height(&self, index: usize) -> Result<(u32, u32)> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok((data.width, data.height))
            }
            MyCow::Redb(txn, timestamp) => {
                let table = txn.open_table(SNAPSHOTS_TABLE)?;
                let access = table
                    .get(&(*timestamp, index as u64))?
                    .context("Index out of bound in snapshot")?;
                let reduced: ReducedData = bitcode::decode(access.value())?;
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
            MyCow::Redb(txn, timestamp) => {
                let table = txn.open_table(SNAPSHOTS_TABLE)?;
                let access = table
                    .get(&(*timestamp, index as u64))?
                    .context("Index out of bound in snapshot")?;
                let reduced: ReducedData = bitcode::decode(access.value())?;
                Ok(reduced.hash)
            }
        }
    }
}
