pub mod new;
pub mod read_rows;
pub mod read_scrollbar;
pub mod read_tree_snapshot;

use std::sync::LazyLock;

use dashmap::DashMap;
use redb::TableDefinition;

use crate::public::structure::response::row::ScrollBarData;

pub const SCROLLBAR_METADATA_TABLE: TableDefinition<i64, &[u8]> =
    TableDefinition::new("scrollbar_metadata");

#[derive(Debug, Clone)]
pub struct PendingTreeSnapshot {
    pub slots: Vec<u64>,
    pub scrollbar: Vec<ScrollBarData>,
}

#[derive(Debug)]
pub struct TreeSnapshot {
    pub in_disk: &'static redb::Database,
    /// Ordered generational arena identities. Static display/query fields are
    /// resolved from `RecordArena`, avoiding a second full metadata copy per
    /// UI snapshot.
    pub in_memory: &'static DashMap<i64, PendingTreeSnapshot>,
}

pub static TREE_SNAPSHOT: LazyLock<TreeSnapshot> = LazyLock::new(TreeSnapshot::new);
