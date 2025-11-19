use std::sync::{LazyLock, Mutex};
use rusqlite::Connection;

pub struct Sqlite {
    pub conn: Mutex<Connection>,
}

impl Sqlite {
    pub fn new() -> Self {
        let path = "./db/sqlite.db";
        let conn = Connection::open(path).expect("Failed to open sqlite db");

        // Enable WAL mode for better concurrency
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .expect("Failed to set PRAGMA");

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                data BLOB
            )",
            [],
        )
        .expect("Failed to create objects table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS albums (
                id TEXT PRIMARY KEY,
                data BLOB
            )",
            [],
        )
        .expect("Failed to create albums table");

        Self {
            conn: Mutex::new(conn),
        }
    }
}

pub static SQLITE: LazyLock<Sqlite> = LazyLock::new(|| Sqlite::new());
