use redb::Database;
use std::sync::LazyLock;

pub static SETTINGS_DB: LazyLock<Database> =
    LazyLock::new(|| Database::create("./db/setting.redb").expect("Failed to create setting.redb"));
