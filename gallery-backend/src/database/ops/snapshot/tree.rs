use crate::common::ROW_BATCH_NUMBER;
use crate::database::ops::snapshot::create_tree::SNAPSHOTS_TABLE;
use crate::models::dto::reduced_data::ReducedData;
use crate::models::entity::row::{DisplayElement, Row};
use anyhow::{Context, Result, bail};
use arrayvec::ArrayString;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use log::error;
use redb::{Database, ReadOnlyTable, ReadTransaction, ReadableDatabase}; // Import ReadOnlyTable
use std::sync::LazyLock;

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

    // 修改：這裡需要傳入 txn，因為 Redb 的 Table 需要借用 txn
    pub fn read_tree_snapshot(
        &'static self,
        txn: &ReadTransaction,
        timestamp: &u128,
    ) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        // 這裡只 open table 一次
        let table = txn.open_table(SNAPSHOTS_TABLE)?;
        Ok(MyCow::Redb(table, *timestamp))
    }

    // read_row 返回的是 Row (Owned data)，所以它可以在內部管理 transaction
    pub fn read_row(&'static self, row_index: usize, timestamp: u128) -> Result<Row> {
        // 1. 開啟 Transaction
        let txn = self.in_disk.begin_read()?;

        // 2. 獲取 Snapshot (這時 MyCow::Redb 會借用上面的 txn)
        let tree_snapshot = self.read_tree_snapshot(&txn, &timestamp)?;

        let data_length = tree_snapshot.len()?; // 注意：len 現在可能回傳 Result
        let chunk_count = (data_length + ROW_BATCH_NUMBER - 1) / ROW_BATCH_NUMBER;

        if row_index > chunk_count {
            error!("read_rows out of bound");
            bail!("Row index out of bounds");
        }

        let start_idx = row_index * ROW_BATCH_NUMBER;
        let end_idx = (start_idx + ROW_BATCH_NUMBER).min(data_length);
        let number_vec = start_idx..end_idx;

        let display_elements: Vec<DisplayElement> = number_vec
            .map(|index| -> Result<DisplayElement> {
                let (width, height) = tree_snapshot.get_width_height(index)?;
                Ok(DisplayElement {
                    display_width: width,
                    display_height: height,
                })
            })
            .collect::<Result<Vec<DisplayElement>>>()?;

        // txn 在這裡結束，DisplayElement 已經被 copy 出來了，安全。
        Ok(Row {
            start: start_idx,
            end: end_idx.saturating_sub(1),
            display_elements,
            row_index: row_index,
        })
    }
}

pub enum MyCow {
    DashMap(Ref<'static, u128, Vec<ReducedData>>),
    // 這裡儲存 ReadOnlyTable，而不是 Transaction
    // key: (u128, u64), value: &'static [u8] (redb 3.x 預設 lifetime)
    Redb(ReadOnlyTable<(u128, u64), &'static [u8]>, u128),
}

impl MyCow {
    pub fn len(&self) -> Result<usize> {
        match self {
            MyCow::DashMap(data) => Ok(data.value().len()),
            MyCow::Redb(table, timestamp) => {
                // 優化：使用 range count 而不是 iter
                let start = (*timestamp, 0);
                let end = (*timestamp, u64::MAX);
                let count = table.range(start..=end)?.count();
                Ok(count)
            }
        }
    }

    pub fn get_width_height(&self, index: usize) -> Result<(u32, u32)> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok((data.width, data.height))
            }
            MyCow::Redb(table, timestamp) => {
                // 直接使用 table.get，無需再次 open_table
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
            MyCow::Redb(table, timestamp) => {
                // 直接使用 table.get，無需再次 open_table
                let access = table
                    .get(&(*timestamp, index as u64))?
                    .context("Index out of bound in snapshot")?;
                let reduced: ReducedData = bitcode::decode(access.value())?;
                Ok(reduced.hash)
            }
        }
    }
}
