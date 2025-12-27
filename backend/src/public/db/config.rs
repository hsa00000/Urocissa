use redb::Database;
use std::sync::LazyLock;

// Refactor: Renamed SETTINGS_DB to CONFIG_DB, file name to config.redb
pub static CONFIG_DB: LazyLock<Database> =
    LazyLock::new(|| Database::create("./db/config.redb").expect("Failed to create config.redb"));
