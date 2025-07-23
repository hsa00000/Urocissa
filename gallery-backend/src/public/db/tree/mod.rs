pub mod new;
pub mod read_tags;

use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use std::sync::{Arc, LazyLock, RwLock, atomic::AtomicU64};

pub struct Tree {
    pub in_disk: &'static redb::Database,
    pub in_memory: &'static Arc<RwLock<Vec<DatabaseTimestamp>>>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree::new());

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
