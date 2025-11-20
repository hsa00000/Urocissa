use rusqlite::Connection;

const CREATE_ALIASES_SQL: &str = include_str!("sql/create_aliases.sql");

pub fn create_aliases_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_ALIASES_SQL, []).map(|_| ())
}
