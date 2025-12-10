use std::sync::LazyLock;
use dashmap::DashMap;
use redb::{Database, ReadTransaction, ReadableDatabase};
use crate::models::dto::reduced_data::ReducedData;
use arrayvec::ArrayString;
use crate::database::ops::snapshot::create_tree;
use crate::database::ops::snapshot::create_tree::SNAPSHOTS_TABLE;
use crate::common::consts::ROW_BATCH_NUMBER;
use crate::models::entity::row::{DisplayElement, Row, ScrollBarData};
use anyhow::{Result, bail, Context};
use log::{error, info};
use chrono::{Datelike, TimeZone, Utc};
use std::time::Instant;
use dashmap::mapref::one::Ref;

#[derive(Debug)]
pub struct TreeSnapshot {
    pub in_disk: &'static Database,
    pub in_memory: &'static DashMap<u128, Vec<ReducedData>>,
}

pub static TREE_SNAPSHOT: LazyLock<TreeSnapshot> = LazyLock::new(|| TreeSnapshot::new());

impl TreeSnapshot {
    pub fn new() -> Self {
        create_tree::create_tree()
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

    pub fn read_scrollbar(&'static self, timestamp: u128) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        let tree_snapshot = self.read_tree_snapshot(&timestamp).unwrap();
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        let mut process_data = |index: usize, data: &ReducedData| {
            let datetime = Utc.timestamp_opt(data.date as i64, 0).unwrap();
            let year = datetime.year();
            let month = datetime.month();
            if last_year != Some(year) || last_month != Some(month) {
                last_year = Some(year);
                last_month = Some(month);
                scroll_bar_data_vec.push(ScrollBarData {
                    year: year as usize,
                    month: month as usize,
                    index: index,
                });
            }
        };

        match tree_snapshot {
            MyCow::DashMap(ref_data) => {
                ref_data.iter().enumerate().for_each(|(index, data)| {
                    process_data(index, data);
                });
            }
            MyCow::Redb(txn, ts) => {
                let table = txn.open_table(SNAPSHOTS_TABLE).unwrap();
                let start = (ts, 0);
                let end = (ts, u64::MAX);
                for (index, entry) in table.range(start..=end).unwrap().enumerate() {
                    let (_, val) = entry.unwrap();
                    if let Ok(data) = bitcode::decode::<ReducedData>(val.value()) {
                        process_data(index, &data);
                    }
                }
            }
        }
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Generate scrollbar");
        scroll_bar_data_vec
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
