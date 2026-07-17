pub mod new;
pub mod read_tags;

use crate::public::structure::response::database_timestamp::DatabaseTimestamp;
use crate::storage::DataStore;
use std::sync::{Arc, LazyLock, RwLock, atomic::AtomicI64};

pub struct Tree {
    pub store: &'static DataStore,
    pub in_memory: &'static Arc<RwLock<Vec<DatabaseTimestamp>>>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(Tree::new);

pub static VERSION_COUNT_TIMESTAMP: AtomicI64 = AtomicI64::new(0);
