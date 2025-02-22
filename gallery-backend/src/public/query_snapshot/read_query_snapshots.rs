use std::{error::Error, sync::atomic::Ordering};

use super::QuerySnapshot;
use crate::public::{query_snapshot::Prefetch, tree::start_loop::VERSION_COUNT_TIMESTAMP};

use redb::TableDefinition;

impl QuerySnapshot {
    pub fn read_query_snapshot(
        &'static self,
        query_hash: u64,
    ) -> Result<Option<Prefetch>, Box<dyn Error>> {
        if let Some(data) = self.in_memory.get(&query_hash) {
            return Ok(Some(data.value().clone()));
        }

        let read_txn = self.in_disk.begin_read().unwrap();

        let count_version = &VERSION_COUNT_TIMESTAMP.load(Ordering::Relaxed).to_string();

        let table_definition: TableDefinition<u64, Prefetch> = TableDefinition::new(&count_version);

        let table = read_txn.open_table(table_definition)?;

        let timestamp = table.get(query_hash)?;

        Ok(timestamp.map(|inner_value| inner_value.value()))
    }
}
