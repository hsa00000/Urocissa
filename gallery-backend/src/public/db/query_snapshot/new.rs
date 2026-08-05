use dashmap::DashMap;
use std::sync::LazyLock;

use super::{Prefetch, QuerySnapshot};

use crate::public::constant::storage::get_data_path;
use crate::storage::cache::{CacheClass, database_builder};

static QUERY_SNAPSHOT_IN_DISK: LazyLock<redb::Database> = LazyLock::new(|| {
    let db_directory = get_data_path().join("db");
    let path = db_directory.join("cache_db_v7.redb");
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).unwrap();
    }
    // Cached Prefetch values reference tree snapshots, so invalidate them when
    // the derived snapshot schema changes.
    for legacy_path in [
        db_directory.join("cache_db.redb"),
        db_directory.join("cache_db_v4.redb"),
        db_directory.join("cache_db_v5.redb"),
        db_directory.join("cache_db_v6.redb"),
    ] {
        if legacy_path.exists()
            && let Err(error) = std::fs::remove_file(&legacy_path)
        {
            log::warn!(
                "failed to remove incompatible query snapshot cache {}: {error}",
                legacy_path.display()
            );
        }
    }
    database_builder(CacheClass::QuerySnapshot)
        .create(path)
        .unwrap()
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
