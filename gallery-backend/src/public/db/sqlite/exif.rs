use rusqlite::Connection;

const CREATE_EXIF_SQL: &str = include_str!("sql/create_exif.sql");

pub fn create_exif_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_EXIF_SQL, []).map(|_| ())
}