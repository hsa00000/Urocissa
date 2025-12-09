use dashmap::DashMap;
use redb::{Database, TableDefinition};
use std::sync::LazyLock;

use crate::public::structure::reduced_data::ReducedData;

use super::TreeSnapshot;

// Key: (Timestamp, RowIndex) -> 使用複合鍵可以自動按 RowIndex 排序
// Value: Serialized ReducedData (blob)
pub const SNAPSHOTS_TABLE: TableDefinition<(u128, u64), &[u8]> = TableDefinition::new("snapshots");

static TREE_SNAPSHOT_IN_DISK: LazyLock<Database> = LazyLock::new(|| {
    let db =
        Database::create("./db/tree_snapshot.redb").expect("Failed to create tree_snapshot.redb");

    // 初始化 Table
    let txn = db.begin_write().unwrap();
    txn.open_table(SNAPSHOTS_TABLE).unwrap();
    txn.commit().unwrap();

    db
});

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<DashMap<u128, Vec<ReducedData>>> =
    LazyLock::new(|| DashMap::new());

impl TreeSnapshot {
    pub fn new() -> Self {
        Self {
            in_disk: &TREE_SNAPSHOT_IN_DISK,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
