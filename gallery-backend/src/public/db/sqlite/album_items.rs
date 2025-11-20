use rusqlite::{Connection, params};

const CREATE_ALBUM_ITEMS_SQL: &str = include_str!("sql/create_album_items.sql");

pub fn create_album_items_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(CREATE_ALBUM_ITEMS_SQL)?;
    Ok(())
}

pub fn _get_objects_in_album(conn: &Connection, album_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT item_id FROM album_items WHERE album_id = ?")?;
    let iter = stmt.query_map(params![album_id], |row| row.get(0))?;
    let mut ids = Vec::new();
    for id in iter {
        ids.push(id?);
    }
    Ok(ids)
}