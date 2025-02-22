pub mod new;
pub mod read_query_snapshots;
pub mod start_loop;
use dashmap::DashMap;
use std::sync::LazyLock;

use crate::router::get::get_prefetch::Prefetch;

#[derive(Debug)]
pub struct QuerySnapshot {
    pub in_disk: &'static redb::Database,
    pub in_memory: &'static DashMap<u64, Prefetch>, // hash of query and VERSION_COUNT_TIMESTAMP -> prefetch
}

pub static QUERY_SNAPSHOT: LazyLock<QuerySnapshot> = LazyLock::new(|| QuerySnapshot::new());
