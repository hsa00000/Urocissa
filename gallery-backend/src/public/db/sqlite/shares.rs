use crate::public::structure::album::Share;
use arrayvec::ArrayString;
use rusqlite::Connection;
use std::collections::HashMap;

const CREATE_SHARES_SQL: &str = include_str!("sql/create_shares.sql");

pub fn create_shares_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_SHARES_SQL, []).map(|_| ())
}

pub fn get_album_shares(
    conn: &Connection,
    album_id: &str,
) -> rusqlite::Result<HashMap<ArrayString<64>, Share>> {
    let mut share_list = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT url, description, password, show_metadata, show_download, show_upload, exp FROM shares WHERE album_id = ?"
    )?;
    let iter = stmt.query_map(rusqlite::params![album_id], |row| {
        let url: String = row.get(0)?;
        let description: String = row.get(1)?;
        let password: Option<String> = row.get(2)?;
        let show_metadata: bool = row.get(3)?;
        let show_download: bool = row.get(4)?;
        let show_upload: bool = row.get(5)?;
        let exp: u64 = row.get(6)?;

        let share = Share {
            url: ArrayString::from(&url).unwrap_or_default(),
            description,
            password,
            show_metadata,
            show_download,
            show_upload,
            exp,
        };
        Ok((url, share))
    })?;
    for result in iter {
        let (url, share) = result?;
        share_list.insert(ArrayString::from(&url).unwrap_or_default(), share);
    }
    Ok(share_list)
}
