use super::Tree;
use crate::public::database_struct::database_timestamp::DatabaseTimestamp;
use std::sync::{Arc, LazyLock, RwLock};

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<Arc<RwLock<Vec<DatabaseTimestamp>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));

static TREE_SNAPSHOT_IN_DISK: LazyLock<redb::Database> =
    LazyLock::new(|| redb::Database::create("./db/index.redb").unwrap());

impl Tree {
    pub fn new() -> Self {
        Self {
            in_disk: &TREE_SNAPSHOT_IN_DISK,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
