# SQLite Migration Plan

This plan outlines the incremental migration from `redb` to `rusqlite`.
The goal is to remove `redb` and its associated caching mechanisms (`tree_snapshots`, `cache_db`, `expire_db`) in favor of direct SQLite queries.

## Phase 1: Infrastructure & Dual Write
*Goal: Establish SQLite presence and ensure new data is persisted to both databases without breaking existing functionality.*

- [x] **Add Dependencies**: Add `rusqlite` (with `bundled` feature) to `Cargo.toml`.
- [x] **Initialize SQLite**:
    - Create `src/public/db/sqlite.rs`.
    - Implement database connection setup (enable WAL mode).
    - Create tables:
        - `objects` (id TEXT PRIMARY KEY, data BLOB)
        - `albums` (id TEXT PRIMARY KEY, data BLOB)
- [x] **Implement Dual Write**:
    - Modify `src/tasks/batcher/flush_tree.rs`.
    - In `flush_tree_task`, insert/delete data into SQLite *in addition to* the existing `redb` operations.
    - Ensure SQLite errors are logged but do not crash the application (initially).

## Phase 2: Migrate Reads & Expand Schema (Incremental)
*Goal: Switch read operations to SQLite and expose data fields for SQL querying.*

- [x] **Migrate Basic Object/Album Lookup**:
    - Identify functions reading single objects/albums from `redb`.
    - Replace implementation to query SQLite `objects` / `albums` tables (using ID).
- [x] **Expand Schema for Querying**:
    - Identify fields needed for sorting/filtering (e.g., `timestamp`, `file_type`, `is_deleted`).
    - Add these columns to `objects` and `albums` tables via `ALTER TABLE` or migration script.
    - Update `flush_tree.rs` (Dual Write) to populate these new columns.
- [x] **Migrate List/Search Operations**:
    - Identify endpoints using `tree_snapshots` for listing or searching.
    - Rewrite these to use raw SQL `SELECT` queries or adapt `TreeSnapshot` to query SQLite.
- [ ] **Migrate Expiration Logic**:
    - If `expire_db` is used for TTL, implement a cleanup task using a simple SQL query (e.g., `DELETE FROM objects WHERE ...`).

## Phase 3: Cleanup & Cutover
*Goal: Remove `redb` and legacy caching code.*

- [x] **Stop Dual Write**:
    - Remove `redb` write logic from `flush_tree.rs` and `flush_tree_snapshot.rs`.
- [x] **Ensure Version Persistence**:
    - Ensure `VERSION_COUNT_TIMESTAMP` initializes from SQLite `snapshots` table on startup.
- [x] **Remove Redb Dependencies**:
    - Remove `redb` from `Cargo.toml`.
    - Delete `src/public/db/tree.rs` (or the redb parts of it).
    - Delete `src/public/constant/redb.rs`.
- [x] **Retire Redb Backend for Snapshots**:
    - Keep `TreeSnapshot` struct as the API interface.
    - Remove `in_disk` (Redb) field from `TreeSnapshot`.
    - Ensure all snapshot logic is pure SQLite.
- [x] **Remove Legacy Caches**:
    - Delete `cache_db` related code.
    - Delete `expire_db` related code.

## Phase 4: Snapshot Optimization & Simplification
*Goal: Simplify snapshot architecture by merging QuerySnapshot into TreeSnapshot and leveraging SQLite for filtering/sorting.*

- [x] **Merge QuerySnapshot into TreeSnapshot**:
    - Treat search results as just another "snapshot" version.
    - Remove `QuerySnapshot` struct and related code.
    - Update search endpoints to generate a `TreeSnapshot` (via SQLite) and return a version ID.
- [x] **Optimize Snapshot Generation**:
    - Replace manual Rust-side filtering/sorting with SQL queries (`WHERE`, `ORDER BY`).
    - Implement `SELECT id FROM objects WHERE ...` to generate snapshot data directly.
- [x] **Simplify TreeSnapshot Structure**:
    - Ensure `TreeSnapshot` is purely a wrapper around `Vec<ID>` (or a mechanism to fetch it).
    - Remove any remaining legacy caching logic.
- [x] **Optimize SQLite Concurrency (r2d2)**:
    - [x] Add `r2d2` and `r2d2_sqlite` dependencies.
    - [x] Replace `Mutex<Connection>` with `r2d2::Pool<SqliteConnectionManager>`.
    - [x] Ensure all writes go through `FlushTreeTask` (Single Writer Principle).
    - [x] Audit codebase for "raw writes" (direct `INSERT/UPDATE/DELETE` outside of tasks) and refactor them to use `FlushTreeTask` or specific Task structs.
- [x] **Implement SQLite Expiration Logic**:
    - Implement the `DELETE FROM objects WHERE ...` logic in `ExpireCheckTask` (currently stubbed).
- [x] **Final Polish**:
    - Verify all tests/flows work with pure SQLite.

## Phase 5: Normalization & Legacy Removal
*Goal: Fully normalize the database schema, remove the JSON BLOBs, and eliminate legacy "self-update" logic by leveraging SQL relations and aggregations.*

- [x] **Schema Normalization (Breaking Change)**:
    - Create `object_tags` table (One-to-Many relation).
    - Create `album_objects` table (Many-to-Many relation).
    - Refactor `objects` and `albums` tables to store all attributes in columns instead of a JSON BLOB.
    - Migrate existing data from JSON BLOBs to the new normalized tables.
- [x] **Refactor Write Logic (`FlushTreeTask`)**:
    - Update `FlushTreeTask` to write to `objects`, `albums`, `object_tags`, and `album_objects` transactionally.
    - Remove JSON serialization logic.
- [x] **Refactor Read Logic**:
    - Update `get_database` and `get_album` to reconstruct structs by joining tables (or using `json_group_array`).
    - Update `get_album_stats` to use SQL `COUNT` and `SUM` on `album_objects` instead of reading a cached value.
- [x] **Remove Legacy Album Logic**:
    - Remove `AlbumSelfUpdateTask` entirely (stats are now calculated on-read via SQL).
    - Remove `album.self_update()` method.
    - Remove `pending` field from `Album` struct if it was only used for this update process. (Kept as it is used for expiration)
- [x] **Cleanup**:
    - Drop the `data` BLOB columns from `objects` and `albums`.
