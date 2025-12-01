pub mod read_query_snapshots;
use dashmap::DashMap;
use log::error;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::LazyLock;

use crate::router::get::get_prefetch::Prefetch;

#[derive(Debug)]
pub struct QuerySnapshot {
    pub in_disk: Pool<SqliteConnectionManager>,
    pub in_memory: DashMap<u64, Prefetch>,
}

impl QuerySnapshot {
    pub fn new() -> Self {
        // 建立 SQLite 連線池
        let manager = SqliteConnectionManager::file("./db/query_snapshot.db");
        let pool = Pool::new(manager).expect("Failed to create SQLite pool");

        let conn = pool.get().expect("Failed to get SQLite connection");

        // 建立資料表
        // query_hash: 查詢雜湊 (PK)
        // data: 序列化後的 Prefetch 資料
        // expires_at: 過期時間戳 (NULL 代表最新版本，尚未過期)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS query_snapshot (
                query_hash INTEGER PRIMARY KEY,
                data BLOB NOT NULL,
                expires_at INTEGER
            )",
            [],
        )
        .expect("Failed to create query_snapshot table");

        // 建立索引加速過期檢查
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_expires_at ON query_snapshot(expires_at)",
            [],
        )
        .expect("Failed to create index on expires_at");

        // 【啟動時清理】: 刪除上次關機遺留的過期資料
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if let Err(e) = conn.execute(
            "DELETE FROM query_snapshot WHERE expires_at IS NOT NULL AND expires_at < ?",
            [now],
        ) {
            error!("Startup cleanup failed: {}", e);
        }

        Self {
            in_disk: pool,
            in_memory: DashMap::new(),
        }
    }
}

pub static QUERY_SNAPSHOT: LazyLock<QuerySnapshot> = LazyLock::new(|| QuerySnapshot::new());
