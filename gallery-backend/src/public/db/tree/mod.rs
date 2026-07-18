pub mod new;
pub mod read_tags;

use crate::public::structure::response::database_timestamp::DatabaseTimestamp;
use crate::storage::DataStore;
use read_tags::TreeListSnapshot;
use std::sync::{Arc, LazyLock, Mutex, RwLock, atomic::AtomicI64};

pub struct Tree {
    pub store: &'static DataStore,
    pub in_memory: &'static Arc<RwLock<Vec<DatabaseTimestamp>>>,
    pub list_snapshot: &'static Arc<RwLock<Option<Arc<TreeListSnapshot>>>>,
    pub list_snapshot_update_lock: &'static Mutex<()>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(Tree::new);

pub static VERSION_COUNT_TIMESTAMP: AtomicI64 = AtomicI64::new(0);
