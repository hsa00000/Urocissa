# Urocissa 項目從 redb 遷移到 rusqlite 的總結

## 項目概述
Urocissa 是一個圖片/視頻庫後端項目，原使用 redb 作為數據庫，現在正在遷移到 rusqlite (SQLite) 以提高兼容性和性能。

## 遷移進展

### 1. 數據庫表創建
- **文件**: `gallery-backend/src/public/structure/database_struct/database/definition.rs`
- **修改**: 添加 `create_database_table` 函數，使用 rusqlite 創建 `database` 表。
- **表結構**: 包含 hash, size, width, height, thumbhash, phash, ext, exif_vec (JSON), tag (JSON), album (JSON), alias (JSON), ext_type, pending。

- **文件**: `gallery-backend/src/public/structure/album/mod.rs`
- **修改**: 添加 `create_album_table` 函數，使用 rusqlite 創建 `album` 表。
- **表結構**: 包含 id, title, created_time, start_time, end_time, last_modified_time, cover, thumbhash, user_defined_metadata (JSON), share_list (JSON), tag (JSON), width, height, item_count, item_size, pending。

### 2. 數據解析 Helper 函數
- **文件**: `gallery-backend/src/public/structure/database_struct/database/definition.rs`
- **修改**: 添加 `Database::from_row` 函數，從 SQLite Row 解析 Database 結構體。

- **文件**: `gallery-backend/src/public/structure/album/mod.rs`
- **修改**: 添加 `Album::from_row` 函數，從 SQLite Row 解析 Album 結構體。

### 3. 更新樹任務 (Update Tree)
- **文件**: `gallery-backend/src/tasks/batcher/update_tree.rs`
- **修改**: 
  - 移除 `open_data_and_album_tables()` 調用。
  - 使用 `Connection::open("gallery.db")` 打開數據庫。
  - 使用 `SELECT * FROM database/album` 查詢所有記錄，並使用 `from_row` 解析。
  - 保持並行處理和內存更新邏輯。

### 4. 刷新樹任務 (Flush Tree)
- **文件**: `gallery-backend/src/tasks/batcher/flush_tree.rs`
- **修改**:
  - 移除 redb 事務。
  - 使用 `Connection::open("gallery.db")`。
  - 對於插入/移除，使用 `INSERT OR REPLACE` 和 `DELETE` SQL 語句。
  - 序列化複雜字段為 JSON。

### 5. 主程序初始化
- **文件**: `gallery-backend/src/main.rs`
- **修改**:
  - 移除 redb 相關導入。
  - 使用 `SELECT COUNT(*) FROM database/album` 獲取條目數並記錄日誌。

### 6. 去重任務 (Deduplicate)
- **文件**: `gallery-backend/src/tasks/actor/deduplicate.rs`
- **修改**:
  - 移除 `open_data_table()`。
  - 使用 `query_row` 查詢現有數據庫條目。
  - 使用 `Database::from_row` 解析。

### 7. 專輯任務 (Album Actor)
- **文件**: `gallery-backend/src/tasks/actor/album.rs`
- **修改**:
  - 移除 redb 事務和表操作。
  - 使用 `Album::from_row` 查詢專輯。
  - 對於更新，使用 `INSERT OR REPLACE`。
  - 對於刪除的專輯，從內存樹收集相關數據，然後更新數據庫。
