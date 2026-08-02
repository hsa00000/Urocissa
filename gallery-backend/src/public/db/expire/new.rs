use std::sync::LazyLock;

use super::Expire;

use crate::public::constant::storage::get_data_path;
use crate::storage::cache::{CacheClass, database_builder};

static EXPIRE_IN_DISK: LazyLock<redb::Database> = LazyLock::new(|| {
    let path = get_data_path().join("db/expire_db.redb");
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).unwrap();
    }
    database_builder(CacheClass::Expire).create(path).unwrap()
});

impl Expire {
    pub fn new() -> Self {
        Expire {
            in_disk: &EXPIRE_IN_DISK,
        }
    }
}
