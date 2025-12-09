use super::{TreeSnapshot, new::SNAPSHOTS_TABLE};
use crate::public::structure::reduced_data::ReducedData;
use anyhow::{Context, Result};
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;
use redb::{ReadTransaction, ReadableTable};

impl TreeSnapshot {
    // 這裡我們返回 MyCow<'static>，因為 Transaction 可以持有 static Database 的引用
    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<MyCow<'static>> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        let txn = self.in_disk.begin_read()?;
        Ok(MyCow::Redb(txn, *timestamp))
    }
}

pub enum MyCow<'a> {
    DashMap(Ref<'static, u128, Vec<ReducedData>>),
    Redb(ReadTransaction<'a>, u128),
}

impl<'a> MyCow<'a> {
    pub fn len(&self) -> usize {
        match self {
            MyCow::DashMap(data) => data.value().len(),
            MyCow::Redb(txn, timestamp) => {
                let table = txn.open_table(SNAPSHOTS_TABLE).unwrap();
                let start = (*timestamp, 0);
                let end = (*timestamp, u64::MAX);
                // 計算該 Timestamp 下有多少行
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
                let access = table.get(&(*timestamp, index as u64))?.context(format!(
                    "Fail to find width/height in tree snapshots for index {}",
                    index
                ))?;

                let reduced: ReducedData =
                    bitcode::decode(access.value()).context("Failed to decode ReducedData")?;

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
                let access = table.get(&(*timestamp, index as u64))?.context(format!(
                    "Fail to find hash in tree snapshots for index {}",
                    index
                ))?;

                let reduced: ReducedData =
                    bitcode::decode(access.value()).context("Failed to decode ReducedData")?;

                Ok(reduced.hash)
            }
        }
    }
}
