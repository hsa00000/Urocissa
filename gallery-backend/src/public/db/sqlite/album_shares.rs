use crate::public::structure::album::Share;
use arrayvec::ArrayString;
use rusqlite::Connection;
use std::collections::HashMap;

const GET_ALBUM_SHARES_SQL: &str = include_str!("sql/get_album_shares.sql");

pub fn create_album_shares_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS album_shares (
            album_id TEXT,
            share_key TEXT,
            share_value TEXT,
            PRIMARY KEY (album_id, share_key),
            FOREIGN KEY (album_id) REFERENCES album_meta(album_id) ON DELETE CASCADE
        )",
        [],
    ).map(|_| ())
}

pub fn get_album_shares(conn: &Connection, album_id: &str) -> rusqlite::Result<HashMap<ArrayString<64>, Share>> {
    let mut share_list = HashMap::new();
    let mut stmt = conn.prepare(GET_ALBUM_SHARES_SQL)?;
    let iter = stmt.query_map(rusqlite::params![album_id], |row| {
        let key: String = row.get(0)?;
        let value_json: String = row.get(1)?;
        let value: Share = serde_json::from_str(&value_json).unwrap_or_default();
        Ok((key, value))
    })?;
    for result in iter {
        let (key, value) = result?;
        share_list.insert(ArrayString::from(&key).unwrap_or_default(), value);
    }
    Ok(share_list)
}