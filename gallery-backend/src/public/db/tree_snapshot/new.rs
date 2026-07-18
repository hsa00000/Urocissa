use dashmap::DashMap;
use std::sync::LazyLock;

use super::TreeSnapshot;

use crate::public::constant::storage::get_data_path;

static TREE_SNAPSHOT_IN_DISK: LazyLock<redb::Database> = LazyLock::new(|| {
    let db_directory = get_data_path().join("db");
    let legacy_path = db_directory.join("temp_db.redb");
    let path = db_directory.join("temp_db_v4.redb");
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }
    }
    // Tree/query snapshots are derived data. Schema 4 stores only SlotRef
    // values, so an older per-record ReducedData cache must not be opened with
    // the new Redb table definition.
    if legacy_path.exists()
        && let Err(error) = std::fs::remove_file(&legacy_path)
    {
        log::warn!(
            "failed to remove incompatible tree snapshot cache {}: {error}",
            legacy_path.display()
        );
    }
    redb::Database::create(path).unwrap()
});

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<DashMap<i64, Vec<u64>>> = LazyLock::new(DashMap::new);

impl TreeSnapshot {
    pub fn new() -> Self {
        Self {
            in_disk: &TREE_SNAPSHOT_IN_DISK,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
