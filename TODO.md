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
- [ ] **Final Polish**:
    - Verify all tests/flows work with pure SQLite.
