use dashmap::DashMap;
use std::sync::LazyLock;

use super::{Prefetch, QuerySnapshot};

static QUERY_SNAPSHOT_IN_MEMORY: LazyLock<DashMap<u64, Prefetch>> =
    LazyLock::new(|| DashMap::new());

impl QuerySnapshot {
    pub fn new() -> Self {
        Self {
            in_memory: &QUERY_SNAPSHOT_IN_MEMORY,
        }
    }
}

