use super::Tree;
use crate::public::db::tree::state::TreeState;
use crate::storage::DataStore;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

static TREE_STATE_IN_MEMORY: LazyLock<Arc<RwLock<TreeState>>> =
    LazyLock::new(|| Arc::new(RwLock::new(TreeState::default())));

static TREE_STATE_UPDATE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static TREE_PERSISTENCE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

use crate::public::constant::storage::get_data_path;

static TREE_STORE: LazyLock<DataStore> = LazyLock::new(|| {
    let path = get_data_path().join("db/index_v6.redb");
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).unwrap();
    }
    DataStore::open(&path).unwrap()
});

impl Tree {
    pub fn new() -> Self {
        Self {
            store: &TREE_STORE,
            state: &TREE_STATE_IN_MEMORY,
            state_update_lock: &TREE_STATE_UPDATE_LOCK,
            persistence_lock: &TREE_PERSISTENCE_LOCK,
        }
    }
}
