use dashmap::DashMap;
use std::sync::LazyLock;

use crate::public::structure::response::reduced_data::ReducedData;

use super::TreeSnapshot;

static TREE_SNAPSHOT_IN_DISK: LazyLock<redb::Database> =
    LazyLock::new(|| redb::Database::create("./db/temp_db.redb").unwrap());

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<DashMap<u128, Vec<ReducedData>>> =
    LazyLock::new(|| DashMap::new());

impl TreeSnapshot {
    pub fn new() -> Self {
        Self {
            in_disk: &TREE_SNAPSHOT_IN_DISK,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
