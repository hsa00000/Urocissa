use std::error::Error;

use super::QuerySnapshot;
use crate::public::db::query_snapshot::Prefetch;

impl QuerySnapshot {
    pub fn read_query_snapshot(
        &'static self,
        query_hash: u64,
    ) -> Result<Option<Prefetch>, Box<dyn Error>> {
        if let Some(data) = self.in_memory.get(&query_hash) {
            return Ok(Some(data.value().clone()));
        }

        Ok(None)
    }
}

