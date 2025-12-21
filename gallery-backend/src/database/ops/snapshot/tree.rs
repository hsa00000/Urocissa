use std::sync::LazyLock;
use dashmap::DashMap;
use redb::{Database, ReadTransaction, ReadableDatabase};
use crate::models::dto::reduced_data::ReducedData;
use arrayvec::ArrayString;
use crate::database::ops::snapshot::create_tree::SNAPSHOTS_TABLE;
use crate::common::ROW_BATCH_NUMBER;
use crate::models::entity::row::{DisplayElement, Row};
use anyhow::{Result, bail, Context};
use log::error;
use dashmap::mapref::one::Ref;

#[derive(Debug)]
pub struct TreeSnapshot {
    pub in_disk: &'static Database,
    pub in_memory: &'static DashMap<u128, Vec<ReducedData>>,
}

pub static TREE_SNAPSHOT: LazyLock<TreeSnapshot> = LazyLock::new(|| TreeSnapshot::new());

impl TreeSnapshot {
    pub fn new() -> Self {
        super::create_tree::create_tree()
    }

    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }
        let txn = self.in_disk.begin_read()?;
        Ok(MyCow::Redb(txn, *timestamp))
    }

    pub fn read_row(&'static self, row_index: usize, timestamp: u128) -> Result<Row> {
        let tree_snapshot = self.read_tree_snapshot(&timestamp)?;

        let data_length = tree_snapshot.len();
        let chunk_count = (data_length + ROW_BATCH_NUMBER - 1) / ROW_BATCH_NUMBER; // Calculate total chunks

        if row_index > chunk_count {
            error!("read_rows out of bound");
            bail!("Row index out of bounds");
        }

        let number_vec = (row_index * ROW_BATCH_NUMBER)
            ..(row_index * ROW_BATCH_NUMBER + ROW_BATCH_NUMBER).min(data_length);

        let display_elements: Vec<DisplayElement> = number_vec
            .map(|index| -> Result<DisplayElement> {
                let (width, height) = tree_snapshot.get_width_height(index)?;
                Ok(DisplayElement {
                    display_width: width,
                    display_height: height,
                })
            })
            .collect::<Result<Vec<DisplayElement>>>()?;

        Ok(Row {
            start: row_index * ROW_BATCH_NUMBER,
            end: row_index * ROW_BATCH_NUMBER + ROW_BATCH_NUMBER - 1,
            display_elements,
            row_index: row_index,
        })
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
