use dashmap::DashMap;
use std::sync::LazyLock;

use super::{PendingTreeSnapshot, SCROLLBAR_METADATA_TABLE, TREE_SNAPSHOT_TABLE, TreeSnapshot};

use crate::public::constant::storage::get_data_path;
use crate::storage::cache::{CacheClass, database_builder};

static TREE_SNAPSHOT_IN_DISK: LazyLock<redb::Database> = LazyLock::new(|| {
    let db_directory = get_data_path().join("db");
    let path = db_directory.join("temp_db_v6.redb");
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }
    }
    // Tree/query snapshots are derived data. Schema 6 stores the ordered
    // ordinals and target bitmap in one compact blob, so old files can be
    // discarded without a durable migration.
    for legacy_path in [
        db_directory.join("temp_db.redb"),
        db_directory.join("temp_db_v4.redb"),
        db_directory.join("temp_db_v5.redb"),
    ] {
        if legacy_path.exists()
            && let Err(error) = std::fs::remove_file(&legacy_path)
        {
            log::warn!(
                "failed to remove incompatible tree snapshot cache {}: {error}",
                legacy_path.display()
            );
        }
    }
    let database = database_builder(CacheClass::TreeSnapshot)
        .create(path)
        .unwrap();
    let txn = database.begin_write().unwrap();
    {
        let _snapshots = txn.open_table(TREE_SNAPSHOT_TABLE).unwrap();
        let _scrollbar = txn.open_table(SCROLLBAR_METADATA_TABLE).unwrap();
    }
    txn.commit().unwrap();
    database
});

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<DashMap<i64, PendingTreeSnapshot>> =
    LazyLock::new(DashMap::new);
static TREE_SNAPSHOT_VERIFIED_LAYOUTS: LazyLock<DashMap<i64, super::SnapshotBlobLayout>> =
    LazyLock::new(DashMap::new);

impl TreeSnapshot {
    pub fn new() -> Self {
        Self {
            in_disk: &TREE_SNAPSHOT_IN_DISK,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
            verified_layouts: &TREE_SNAPSHOT_VERIFIED_LAYOUTS,
        }
    }
}
