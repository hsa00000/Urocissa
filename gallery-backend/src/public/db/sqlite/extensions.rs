use rusqlite::Connection;

pub fn create_extensions_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS extensions (
            node_id TEXT PRIMARY KEY,
            ext TEXT,
            FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
        )",
        [],
    ).map(|_| ())
}