use redb::Database;

use super::Tree;
use crate::models::entity::abstract_data::AbstractData;
use std::sync::{Arc, LazyLock, RwLock};

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<Arc<RwLock<Vec<AbstractData>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));

impl Tree {
    pub fn new() -> Self {
        let db = Database::create("./db/gallery.redb").expect("Failed to create gallery.redb");
        Self {
            in_disk: db,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
