use arrayvec::ArrayString;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::public::db::tree::TREE;

/// Share: 用於前端傳輸的分享結構
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub url: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: u64,
}

/// ResolvedShare: 包含 album 資訊的完整分享結構
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedShare {
    pub share: Share,
    pub album_id: ArrayString<64>,
    pub album_title: Option<String>,
}

impl ResolvedShare {
    pub fn new(album_id: ArrayString<64>, album_title: Option<String>, share: Share) -> Self {
        Self {
            share,
            album_id,
            album_title,
        }
    }
}

pub struct AlbumShareTable;

impl AlbumShareTable {
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

    pub fn get_all_shares_grouped()
    -> rusqlite::Result<HashMap<String, HashMap<ArrayString<64>, Share>>> {
        let conn = TREE.get_connection().unwrap();
        let mut stmt = conn.prepare("SELECT album_id, url, description, password, show_metadata, show_download, show_upload, exp FROM album_share")?;

        let share_iter = stmt.query_map([], |row| {
            let album_id: String = row.get(0)?;
            let url_str: String = row.get(1)?;
            let url = ArrayString::from(&url_str).unwrap();

            Ok((
                album_id,
                url,
                Share {
                    url,
                    description: row.get(2)?,
                    password: row.get(3)?,
                    show_metadata: row.get(4)?,
                    show_download: row.get(5)?,
                    show_upload: row.get(6)?,
                    exp: row.get(7)?,
                },
            ))
        })?;

        let mut map: HashMap<String, HashMap<ArrayString<64>, Share>> = HashMap::new();

        for share_result in share_iter {
            if let Ok((album_id, url, share)) = share_result {
                map.entry(album_id).or_default().insert(url, share);
            }
        }

        Ok(map)
    }

    pub fn get_all_resolved() -> rusqlite::Result<Vec<ResolvedShare>> {
        let conn = TREE.get_connection().unwrap();
        let sql = r#"
            SELECT 
                s.url, s.description, s.password, s.show_metadata, 
                s.show_download, s.show_upload, s.exp,
                s.album_id, a.title
            FROM album_share s
            LEFT JOIN album a ON s.album_id = a.id
        "#;

        let mut stmt = conn.prepare(sql)?;
        let share_iter = stmt.query_map([], |row| {
            let url_str: String = row.get(0)?;
            let url = ArrayString::from(&url_str).unwrap();

            let album_id_str: String = row.get(7)?;
            let album_id = ArrayString::from(&album_id_str).unwrap();

            Ok(ResolvedShare {
                share: Share {
                    url,
                    description: row.get(1)?,
                    password: row.get(2)?,
                    show_metadata: row.get(3)?,
                    show_download: row.get(4)?,
                    show_upload: row.get(5)?,
                    exp: row.get(6)?,
                },
                album_id,
                album_title: row.get(8)?,
            })
        })?;

        let mut shares = Vec::new();
        for share in share_iter {
            if let Ok(s) = share {
                shares.push(s);
            }
        }
        Ok(shares)
    }
}
