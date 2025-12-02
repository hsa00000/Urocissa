use dashmap::DashMap;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::LazyLock;

use crate::public::structure::reduced_data::ReducedData;

use super::TreeSnapshot;

static TREE_SNAPSHOT_IN_DISK: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    let manager = SqliteConnectionManager::file("./db/tree_snapshot.db");
    let pool = Pool::new(manager).expect("Failed to create tree_snapshot DB pool");

    let conn = pool.get().expect("Failed to get tree_snapshot connection");
    
    // 建立資料表
    // timestamp: 快照時間戳 (String)
    // row_index: 行索引 (Integer)
    // data: ReducedData 的二進位資料 (Blob)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snapshots (
            timestamp TEXT NOT NULL,
            row_index INTEGER NOT NULL,
            data BLOB NOT NULL,
            PRIMARY KEY (timestamp, row_index)
        )",
        [],
    )
    .expect("Failed to create snapshots table");

    pool
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
