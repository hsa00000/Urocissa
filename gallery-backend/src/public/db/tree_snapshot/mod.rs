pub mod new;
pub mod read_rows;
pub mod read_scrollbar;
pub mod read_tree_snapshot;

use std::sync::LazyLock;

use dashmap::DashMap;

#[derive(Debug)]
pub struct TreeSnapshot {
    pub in_disk: &'static redb::Database,
    /// Ordered generational arena identities. Static display/query fields are
    /// resolved from `RecordArena`, avoiding a second full metadata copy per
    /// UI snapshot.
    pub in_memory: &'static DashMap<i64, Vec<u64>>,
}

pub static TREE_SNAPSHOT: LazyLock<TreeSnapshot> = LazyLock::new(TreeSnapshot::new);
