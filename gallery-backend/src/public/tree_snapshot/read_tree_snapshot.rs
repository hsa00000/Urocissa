use super::TreeSnapshot;
use crate::public::reduced_data::ReducedData;
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;

use redb::{ReadOnlyTable, ReadableTableMetadata, TableDefinition, TableError};
use rocket::http::Status;

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<MyCow, Status> {
        if let Some(data) = self.in_memory.get(timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        let read_txn = self.in_disk.begin_read().map_err(|err| {
            error!("{:#?}", err);
            Status::InternalServerError
        })?;

        let binding = timestamp.to_string();
        let table_definition: TableDefinition<u64, ReducedData> = TableDefinition::new(&binding);

        let table = read_txn
            .open_table(table_definition)
            .map_err(|err| match err {
                TableError::TableDoesNotExist(_) => {
                    warn!("Table does not exist. Return unauthorized");
                    Status::Unauthorized
                }
                _ => {
                    error!("{:#?}", err);
                    Status::InternalServerError
                }
            })?;
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

    pub fn get_width_height(&self, index: usize) -> (u32, u32) {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                (data.width, data.height)
            }
            MyCow::Redb(table) => {
                let data = &table.get(index as u64).unwrap().unwrap().value();
                (data.width, data.height)
            }
        }
    }

    pub fn get_hash(&self, index: usize) -> ArrayString<64> {
        match self {
            MyCow::DashMap(data) => {
                let data = &data.value()[index];
                data.hash
            }
            MyCow::Redb(table) => {
                let data = table.get(index as u64).unwrap().unwrap().value();
                data.hash
            }
        }
    }
}
