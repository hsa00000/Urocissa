pub mod new;
pub mod read_rows;
pub mod read_scrollbar;
pub mod read_tags;
pub mod read_tree_snapshot;

use std::sync::LazyLock;

use dashmap::DashMap;

use crate::public::structure::response::reduced_data::ReducedData;

#[derive(Debug)]
pub struct TreeSnapshot {
    pub in_disk: &'static redb::Database,
    pub in_memory: &'static DashMap<u128, Vec<ReducedData>>,
}

pub static TREE_SNAPSHOT: LazyLock<TreeSnapshot> = LazyLock::new(|| TreeSnapshot::new());
