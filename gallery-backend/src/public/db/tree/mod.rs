pub mod read_tags;

use std::sync::{LazyLock, atomic::AtomicU64};

pub struct Tree;

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree);

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
