use super::Tree;
use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use std::sync::{Arc, LazyLock, RwLock};

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<Arc<RwLock<Vec<DatabaseTimestamp>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));

impl Tree {
    pub fn new() -> Self {
        Self {
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
