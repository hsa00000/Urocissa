use super::QuerySnapshot;
use crate::public::db::query_snapshot::Prefetch;
use crate::workflow::processors::transitor::get_current_timestamp_u64;
use rusqlite::OptionalExtension;
use std::error::Error; // 需確認 rusqlite 有開啟此功能，或手動處理 Error

impl QuerySnapshot {
    pub fn read_query_snapshot(
        &'static self,
        query_hash: u64,
    ) -> Result<Option<Prefetch>, Box<dyn Error>> {
        // 1. Level 1: Check Memory (DashMap)
        if let Some(data) = self.in_memory.get(&query_hash) {
            return Ok(Some(data.value().clone()));
        }

        // 2. Level 2: Check Disk (SQLite)
        let conn = self.in_disk.get()?;
        let current_time = get_current_timestamp_u64();

        // 關鍵 SQL：只撈取 (沒過期 OR 還未設定過期時間) 的資料
        let mut stmt = conn.prepare(
            "SELECT data FROM query_snapshot 
             WHERE query_hash = ? 
             AND (expires_at IS NULL OR expires_at > ?)",
        )?;

        // 使用 query_row 搭配 optional() 處理找不到的情況
        // 需要 import rusqlite::OptionalExtension
        let result = stmt
            .query_row([query_hash, current_time], |row| {
                let data: Vec<u8> = row.get(0)?;
                Ok(data)
            })
            .optional()?;

        if let Some(data) = result {
            // 解碼 (假設使用 bitcode，與你原專案一致)
            let prefetch: Prefetch = bitcode::decode(&data)?;

            // 3. Promote to Memory (回補機制)
            // 這樣下次讀取就會命中記憶體，直到 FlushTask 把它清掉或重啟
            self.in_memory.insert(query_hash, prefetch.clone());

            return Ok(Some(prefetch));
        }

        Ok(None)
    }
}
