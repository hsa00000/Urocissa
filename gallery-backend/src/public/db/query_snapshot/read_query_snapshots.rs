use super::{QUERY_SNAPSHOT_TABLE, QuerySnapshot};
use crate::router::get::get_prefetch::Prefetch;
use crate::workflow::processors::transitor::get_current_timestamp_u64;
use redb::ReadableTable;
use serde::{Deserialize, Serialize};
use std::error::Error;

// 定義一個包裝結構來儲存資料與過期時間
// 注意：寫入端也必須使用此結構進行序列化
#[derive(Serialize, Deserialize)]
pub struct StoredSnapshot {
    pub prefetch: Prefetch,
    pub expires_at: Option<u64>,
}

impl QuerySnapshot {
    pub fn read_query_snapshot(
        &'static self,
        query_hash: u64,
    ) -> Result<Option<Prefetch>, Box<dyn Error>> {
        // 1. Level 1: Check Memory (DashMap)
        if let Some(data) = self.in_memory.get(&query_hash) {
            return Ok(Some(data.value().clone()));
        }

        // 2. Level 2: Check Disk (Redb)
        let read_txn = self.in_disk.begin_read()?;
        let table = read_txn.open_table(QUERY_SNAPSHOT_TABLE)?;

        if let Some(access) = table.get(query_hash)? {
            // 使用 bitcode 解碼 (假設原本是用 bitcode，這裡改為 decode StoredSnapshot)
            // 如果舊資料不是這個格式，這裡會報錯，建議清除舊 DB
            let stored: StoredSnapshot = bitcode::decode(access.value())?;

            // 檢查過期
            let current_time = get_current_timestamp_u64();
            if let Some(expires_at) = stored.expires_at {
                if expires_at <= current_time {
                    // 已過期，視為不存在 (依賴後續 Cleanup 刪除)
                    return Ok(None);
                }
            }

            // 3. Promote to Memory
            self.in_memory.insert(query_hash, stored.prefetch.clone());

            return Ok(Some(stored.prefetch));
        }

        Ok(None)
    }
}
