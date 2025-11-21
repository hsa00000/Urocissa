use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::LazyLock;

pub static DB_POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    let manager = SqliteConnectionManager::file("gallery.db");
    Pool::new(manager).expect("Failed to create DB pool")
});