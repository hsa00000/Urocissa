use crate::public::db::tree::TREE;
use crate::public::structure::album::Share;
use arrayvec::ArrayString;
use rusqlite::Connection;
use std::collections::HashMap;

pub struct AlbumShare;

impl AlbumShare {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS album_share (
                album_id TEXT NOT NULL,
                url TEXT NOT NULL,
                description TEXT NOT NULL,
                password TEXT,
                show_metadata INTEGER NOT NULL,
                show_download INTEGER NOT NULL,
                show_upload INTEGER NOT NULL,
                exp INTEGER NOT NULL,
                PRIMARY KEY (album_id, url),
                FOREIGN KEY (album_id) REFERENCES album(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_album_share_album_id
                ON album_share(album_id);
            
            CREATE INDEX IF NOT EXISTS idx_album_share_url
                ON album_share(url);
        "#;
        conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn get_map(album_id: &str) -> rusqlite::Result<HashMap<ArrayString<64>, Share>> {
        let conn = TREE.get_connection().unwrap();
        let mut stmt = conn.prepare("SELECT url, description, password, show_metadata, show_download, show_upload, exp FROM album_share WHERE album_id = ?")?;

        let share_iter = stmt.query_map([album_id], |row| {
            let url_str: String = row.get(0)?;
            let url = ArrayString::from(&url_str).unwrap();
            Ok((
                url,
                Share {
                    url,
                    description: row.get(1)?,
                    password: row.get(2)?,
                    show_metadata: row.get(3)?,
                    show_download: row.get(4)?,
                    show_upload: row.get(5)?,
                    exp: row.get(6)?,
                },
            ))
        })?;

        let mut map = HashMap::new();
        for share in share_iter {
            if let Ok((url, share)) = share {
                map.insert(url, share);
            }
        }
        Ok(map)
    }
}
