use super::{QUERY_SNAPSHOT_TABLE, QuerySnapshot};
use crate::api::handlers::media::Prefetch;
use crate::background::processors::transitor::get_current_timestamp_u64;
use redb::ReadableDatabase;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct StoredSnapshot {
    pub prefetch: Prefetch,
    pub expires_at: Option<u64>,
}

impl QuerySnapshot {
    pub fn read_query_snapshot(
        &'static self,
        query_hash: u64,
    ) -> Result<Option<Prefetch>, Box<dyn Error>> {
        if let Some(data) = self.in_memory.get(&query_hash) {
            return Ok(Some(data.value().clone()));
        }

        let read_txn = self.in_disk.begin_read()?;
        let table = read_txn.open_table(QUERY_SNAPSHOT_TABLE)?;

        if let Some(access) = table.get(query_hash)? {
            let stored: StoredSnapshot = bitcode::decode(access.value())?;

            let current_time = get_current_timestamp_u64();
            if let Some(expires_at) = stored.expires_at {
                if expires_at <= current_time {
                    return Ok(None);
                }
            }

            self.in_memory.insert(query_hash, stored.prefetch.clone());
            return Ok(Some(stored.prefetch));
        }

        Ok(None)
    }
}
