use dashmap::DashMap;
use redb::{Database, ReadableTable, TableDefinition, ReadableDatabase};
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::background::processors::transitor::get_current_timestamp_u64;

use crate::api::handlers::media::Prefetch;

// Key: QueryHash, Value: Serialized StoredSnapshot
pub const QUERY_SNAPSHOT_TABLE: TableDefinition<u64, &[u8]> =
    TableDefinition::new("query_snapshot");

// Key: (ExpiresAt, QueryHash), Value: ()
pub const QUERY_EXPIRY_TABLE: TableDefinition<(u64, u64), ()> =
    TableDefinition::new("query_expiry");

#[derive(Debug)]
pub struct QuerySnapshot {
    pub in_disk: Database,
    pub in_memory: DashMap<u64, Prefetch>,
}

#[derive(Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct StoredSnapshot {
    pub prefetch: Prefetch,
    pub expires_at: Option<u64>,
}

impl QuerySnapshot {
    pub fn new() -> Self {
        let db = Database::create("./db/query_snapshot.redb")
            .expect("Failed to create query_snapshot.redb");

        // 初始化表格
        let write_txn = db.begin_write().unwrap();
        {
            let _ = write_txn.open_table(QUERY_SNAPSHOT_TABLE).unwrap();
            let _ = write_txn.open_table(QUERY_EXPIRY_TABLE).unwrap();
        }
        write_txn.commit().unwrap();

        // 啟動時清理過期資料
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let write_txn = db.begin_write().unwrap();
        {
            let mut data_table = write_txn.open_table(QUERY_SNAPSHOT_TABLE).unwrap();
            let mut expiry_table = write_txn.open_table(QUERY_EXPIRY_TABLE).unwrap();

            let to_delete: Vec<(u64, u64)> = expiry_table
                .range(..=(now, u64::MAX))
                .unwrap()
                .map(|r| {
                    let (key_guard, _) = r.unwrap();
                    let key = key_guard.value();
                    key
                })
                .collect();

            for key in to_delete {
                expiry_table.remove(key).unwrap();
                data_table.remove(key.1).unwrap();
            }
        }
        write_txn.commit().unwrap();

        QuerySnapshot {
            in_disk: db,
            in_memory: DashMap::new(),
        }
    }

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

pub static QUERY_SNAPSHOT: LazyLock<QuerySnapshot> = LazyLock::new(|| QuerySnapshot::new());

