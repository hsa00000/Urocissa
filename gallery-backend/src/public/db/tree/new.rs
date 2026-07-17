use super::Tree;
use crate::public::structure::response::database_timestamp::DatabaseTimestamp;
use crate::storage::DataStore;
use std::sync::{Arc, LazyLock, RwLock};

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<Arc<RwLock<Vec<DatabaseTimestamp>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));

use crate::public::constant::storage::get_data_path;

static TREE_STORE: LazyLock<DataStore> = LazyLock::new(|| {
    let path = get_data_path().join("db/index_v6.redb");
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap();
        }
    }
    DataStore::open(&path).unwrap()
});

impl Tree {
    pub fn new() -> Self {
        Self {
            store: &TREE_STORE,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
