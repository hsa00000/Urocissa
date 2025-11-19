use dashmap::DashMap;
use std::sync::LazyLock;

use crate::public::structure::reduced_data::ReducedData;

use super::TreeSnapshot;

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<DashMap<u128, Vec<ReducedData>>> =
    LazyLock::new(|| DashMap::new());

impl TreeSnapshot {
    pub fn new() -> Self {
        Self {
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}

