pub mod new;
pub mod read_rows;
pub mod read_scrollbar;
pub mod read_tree_snapshot;


use std::sync::LazyLock;

#[derive(Debug)]
pub struct TreeSnapshot;


pub static TREE_SNAPSHOT: LazyLock<TreeSnapshot> = LazyLock::new(|| TreeSnapshot::new());
