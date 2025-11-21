use rusqlite::Connection;
use std::time::Duration;

pub fn init_db_file_once() -> anyhow::Result<()> {
    let conn = Connection::open("./db/gallery.db")?;
    conn.busy_timeout(Duration::from_millis(5000))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}
