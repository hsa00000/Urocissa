# SQLite Migration Plan

This plan outlines the incremental migration from `redb` to `rusqlite`.
The goal is to remove `redb` and its associated caching mechanisms (`tree_snapshots`, `cache_db`, `expire_db`) in favor of direct SQLite queries.

## Phase 1: Infrastructure & Dual Write
*Goal: Establish SQLite presence and ensure new data is persisted to both databases without breaking existing functionality.*

- [ ] **Add Dependencies**: Add `rusqlite` (with `bundled` feature) to `Cargo.toml`.
- [ ] **Initialize SQLite**:
    - Create `src/public/db/sqlite.rs`.
    - Implement database connection setup (enable WAL mode).
    - Create tables:
        - `objects` (id TEXT PRIMARY KEY, data BLOB)
        - `albums` (id TEXT PRIMARY KEY, data BLOB)
- [ ] **Implement Dual Write**:
    - Modify `src/tasks/batcher/flush_tree.rs`.
    - In `flush_tree_task`, insert/delete data into SQLite *in addition to* the existing `redb` operations.
    - Ensure SQLite errors are logged but do not crash the application (initially).

## Phase 2: Data Backfill (Temporary)
*Goal: Populate SQLite with existing data so we can switch reads safely.*

- [ ] **Create Migration Task**:
    - Create a temporary function/task that runs on startup.
    - Iterate through all records in `redb` (`DATA_TABLE` and `ALBUM_TABLE`).
    - Insert them into SQLite if they don't exist.
    - *Note: This ensures that when we switch reads, the data is there.*

## Phase 3: Migrate Reads (Incremental)
*Goal: Switch read operations from `redb`/`tree_snapshots` to SQLite one by one.*

- [ ] **Migrate Basic Object/Album Lookup**:
    - Identify functions reading single objects/albums from `redb`.
    - Replace implementation to query SQLite `objects` / `albums` tables.
- [ ] **Migrate List/Search Operations (Remove Tree Snapshots)**:
    - Identify endpoints using `tree_snapshots` for listing or searching.
    - Rewrite these to use raw SQL `SELECT` queries with `WHERE` clauses on the `objects` table.
    - *Note: This replaces the complex snapshot logic with simple SQL queries.*
- [ ] **Migrate Expiration Logic**:
    - If `expire_db` is used for TTL, implement a cleanup task using a simple SQL query (e.g., `DELETE FROM objects WHERE ...`).

## Phase 4: Cleanup & Cutover
*Goal: Remove `redb` and legacy caching code.*

- [ ] **Stop Dual Write**:
    - Remove `redb` write logic from `flush_tree.rs`.
- [ ] **Remove Redb Dependencies**:
    - Remove `redb` from `Cargo.toml`.
    - Delete `src/public/db/tree.rs` (or the redb parts of it).
    - Delete `src/public/constant/redb.rs`.
- [ ] **Remove Legacy Caches**:
    - Delete `tree_snapshots` related code.
    - Delete `cache_db` related code.
    - Delete `expire_db` related code.
    - Remove `VERSION_COUNT_TIMESTAMP` and `TREE_SNAPSHOT` global variables.
- [ ] **Final Polish**:
    - Remove the temporary migration task from Phase 2.
    - Verify all tests/flows work with pure SQLite.
