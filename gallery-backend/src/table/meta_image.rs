use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadataSchema {
    pub id: ArrayString<64>, // FK to object.id
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub phash: Option<Vec<u8>>,
}

impl ImageMetadataSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS meta_image (
                id TEXT PRIMARY KEY,
                size INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                ext TEXT NOT NULL,
                phash BLOB,
                FOREIGN KEY(id) REFERENCES object(id) ON DELETE CASCADE
            );
        "#;
        conn.execute(sql, [])?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id_str: String = row.get("id")?;
        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            size: row.get("size")?,
            width: row.get("width")?,
            height: row.get("height")?,
            ext: row.get("ext")?,
            phash: row.get("phash")?,
        })
    }

}