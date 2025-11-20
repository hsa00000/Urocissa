use rusqlite::Connection;

const CREATE_EXTENSIONS_SQL: &str = include_str!("sql/create_extensions.sql");

pub fn create_extensions_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_EXTENSIONS_SQL, []).map(|_| ())
}