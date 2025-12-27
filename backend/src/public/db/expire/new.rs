use std::sync::LazyLock;

use super::Expire;

static EXPIRE_IN_DISK: LazyLock<redb::Database> =
    LazyLock::new(|| redb::Database::create("./db/expire_db.redb").unwrap());

impl Expire {
    pub fn new() -> Self {
        Expire {
            in_disk: &EXPIRE_IN_DISK,
        }
    }
}
