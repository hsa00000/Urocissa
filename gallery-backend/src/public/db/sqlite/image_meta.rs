use rusqlite::Connection;

const CREATE_IMAGE_META_SQL: &str = include_str!("sql/create_image_meta.sql");

pub fn create_image_meta_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_IMAGE_META_SQL, []).map(|_| ())
}
