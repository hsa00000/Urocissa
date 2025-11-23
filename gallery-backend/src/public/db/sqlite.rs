use rusqlite::Connection;
use std::time::Duration;

pub fn init_db_file_once() -> anyhow::Result<()> {
    std::fs::create_dir_all("./db")?;
    let conn = Connection::open("./db/gallery.db")?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )?;
    crate::public::db::schema::create_all_tables(&conn)?;
    Ok(())
}
