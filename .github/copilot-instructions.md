你用 agent mode 自主完成任務。
規則：

需要跑指令時，先選擇只讀或可逆指令（如 ls/find/grep/cargo check/）。

不要使用 npm run build 而是要使用 npx vue-tsc --noEmit

請持續迭代直到：編譯/測試通過或你需要我輸入/決策為止；不要中途停下來問「要不要繼續」。

更改資料庫結構時不需要幫我製作遷移腳本。你總是可以假設我的資料庫是空的，是新的。

使用繁體中文回答。

## 專案架構

Urocissa 是自架圖庫應用，後端 Rust (Rocket + redb 嵌入式資料庫)，前端 Vue 3 + Vuetify。核心資料結構為 AbstractData enum (Image/Video/Album)，使用 Expression enum 處理布林搜尋過濾。後端 API 路由分為 media/albums/shares/auth/system，背景任務處理索引與監控。資料庫使用 redb，序列化用 bitcode，ID 用 ArrayString<64>。

## 關鍵工作流程

- 後端開發：`cargo run` (dev) 或 `cargo run --release` (prod)
- 前端開發：`npm run dev` (Vite dev server，代理到後端 5673 埠)
- 建置前端：`npm run build` (包含 `vue-tsc --noEmit` 型別檢查)
- 型別檢查：`npx vue-tsc --noEmit` (不建置，只檢查)
- 測試：無自動化測試，手動驗證
- 更新依賴：後端 `cargo update`，前端 `npm update`

## 專案慣例

- 搜尋語法：使用 Expression (Or/And/Not/Tag/Ext/Model/Make/Path/Album/Any)，參考 SEARCH.md
- 隱私過濾：generate_filter_hide_metadata 方法處理共享相簿隱藏邏輯
- 認證：JWT claims 用於檔案 token，axios 攔截器處理 x-album-id/x-share-id 標頭
- 資料存取：TREE 靜態變數存取 redb 資料庫，bitcode 編碼/解碼
- 前端別名：@ 指向 src，@Menu/@worker/@utils/@type 等
- 錯誤處理：anyhow crate，handle_error 函數

## 整合點

- 開發代理：Vite 代理 /json/assets/put/delete/edit\_\* 到 http://127.0.0.1:5673
- 檔案服務：Rocket FileServer 從 ../gallery-frontend/dist/assets 提供靜態資源
- 背景任務：使用 tokio spawn，crossbeam-queue 處理批次更新
- 虛擬捲動：前端使用 TanStack/virtual 克服 DOM 高度限制

參考檔案：README.md (總覽)，Cargo.toml (後端依賴)，package.json (前端依賴)，main.rs (後端入口)，abstract_data.rs (核心資料結構)，privacy.rs (過濾邏輯)，vite.config.ts (前端配置)。
