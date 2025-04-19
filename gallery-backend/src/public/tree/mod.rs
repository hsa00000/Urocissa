pub mod new;
pub mod read_tags;
pub mod read_tree;
pub mod start_loop;

use crate::public::database_struct::database_timestamp::DatabaseTimestamp;
use std::sync::{Arc, LazyLock, RwLock};

pub struct Tree {
    pub in_disk: &'static redb::Database,
    pub in_memory: &'static Arc<RwLock<Vec<DatabaseTimestamp>>>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree::new());
