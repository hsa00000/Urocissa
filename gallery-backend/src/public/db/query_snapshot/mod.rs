pub mod read_query_snapshots;
use dashmap::DashMap;
use log::error;
use redb::{Database, ReadableTable, TableDefinition};
use std::sync::LazyLock;

use crate::router::get::get_prefetch::Prefetch;

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
                    let (exp, hash) = key;
                    (exp, hash)
                })
                .collect();

            if !to_delete.is_empty() {
                for (exp, hash) in to_delete {
                    expiry_table.remove((exp, hash)).unwrap();
                    data_table.remove(hash).unwrap();
                }
            }
        }
        write_txn.commit().unwrap();

        Self {
            in_disk: db,
            in_memory: DashMap::new(),
        }
    }
}

pub static QUERY_SNAPSHOT: LazyLock<QuerySnapshot> = LazyLock::new(|| QuerySnapshot::new());
