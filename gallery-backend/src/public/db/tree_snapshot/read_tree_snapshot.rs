use super::TreeSnapshot;
use crate::public::structure::response::reduced_data::ReducedData;
use anyhow::Context;
use anyhow::Result;
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;
use redb::{ReadOnlyTable, ReadableDatabase, ReadableTableMetadata, TableDefinition};

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        let read_txn = self.in_disk.begin_read()?;

        let binding = timestamp.to_string();
        let table_definition: TableDefinition<u64, ReducedData> = TableDefinition::new(&binding);

        let table = read_txn.open_table(table_definition)?;
        Ok(MyCow::Redb(table))
    }
}

#[derive(Debug)]
pub enum MyCow {
    DashMap(Ref<'static, u128, Vec<ReducedData>>),
    Redb(ReadOnlyTable<u64, ReducedData>),
}

impl MyCow {
    pub fn len(&self) -> usize {
        match self {
            MyCow::DashMap(data) => data.value().len(),
            MyCow::Redb(table) => table.len().unwrap() as usize,
        }
    }

    pub fn get_width_height(&self, index: usize) -> Result<(u32, u32)> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok((data.width, data.height))
            }
            MyCow::Redb(table) => {
                let data = &table
                    .get(index as u64)?
                    .context(format!(
                        "Fail to find with and height in tree snapshots for index {}",
                        index
                    ))?
                    .value();

                Ok((data.width, data.height))
            }
        }
    }

    pub fn get_hash(&self, index: usize) -> Result<ArrayString<64>> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                Ok(data.hash)
            }
            MyCow::Redb(table) => {
                let data = table
                    .get(index as u64)?
                    .context(format!(
                        "Fail to find hash in tree snapshots for index {}",
                        index
                    ))?
                    .value();
                Ok(data.hash)
            }
        }
    }
}
