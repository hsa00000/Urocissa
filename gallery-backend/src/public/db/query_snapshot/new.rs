use dashmap::DashMap;
use std::sync::LazyLock;

use super::{Prefetch, QuerySnapshot};

use crate::public::constant::storage::get_data_path;

static QUERY_SNAPSHOT_IN_DISK: LazyLock<redb::Database> = LazyLock::new(|| {
    let db_directory = get_data_path().join("db");
    let legacy_path = db_directory.join("cache_db.redb");
    let path = db_directory.join("cache_db_v4.redb");
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }
    }
    // Cached Prefetch values reference tree snapshot tables, so invalidate the
    // old query cache together with the schema-3 tree snapshot representation.
    if legacy_path.exists()
        && let Err(error) = std::fs::remove_file(&legacy_path)
    {
        log::warn!(
            "failed to remove incompatible query snapshot cache {}: {error}",
            legacy_path.display()
        );
    }
    redb::Database::create(path).unwrap()
});

static QUERY_SNAPSHOT_IN_MEMORY: LazyLock<DashMap<u64, Prefetch>> = LazyLock::new(DashMap::new);

impl QuerySnapshot {
    pub fn new() -> Self {
        Self {
            in_disk: &QUERY_SNAPSHOT_IN_DISK,
            in_memory: &QUERY_SNAPSHOT_IN_MEMORY,
        }
    }
}
