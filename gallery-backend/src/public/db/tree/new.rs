use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use super::Tree;
use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use std::sync::{Arc, LazyLock, RwLock};

static TREE_SNAPSHOT_IN_MEMORY: LazyLock<Arc<RwLock<Vec<DatabaseTimestamp>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));

impl Tree {
    pub fn new() -> Self {
        let manager = SqliteConnectionManager::file("./db/gallery.db").with_init(|c| {
            c.execute_batch(
                "PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 5000;",
            )
        });

        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create DB pool");
        Self {
            in_disk: pool,
            in_memory: &TREE_SNAPSHOT_IN_MEMORY,
        }
    }
}
