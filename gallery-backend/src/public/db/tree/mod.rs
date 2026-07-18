pub mod new;
pub mod read_tags;
pub mod state;

use crate::storage::DataStore;
use read_tags::TreeListSnapshot;
use state::TreeState;
use std::sync::{Arc, LazyLock, Mutex, RwLock, atomic::AtomicI64};

pub struct Tree {
    pub store: &'static DataStore,
    pub state: &'static Arc<RwLock<TreeState>>,
    pub list_snapshot: &'static Arc<RwLock<Option<Arc<TreeListSnapshot>>>>,
    pub list_snapshot_update_lock: &'static Mutex<()>,
    pub persistence_lock: &'static Mutex<()>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(Tree::new);

pub static VERSION_COUNT_TIMESTAMP: AtomicI64 = AtomicI64::new(0);
