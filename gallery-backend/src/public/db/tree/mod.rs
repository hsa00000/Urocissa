pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::{HashMap, HashSet};

use crate::public::structure::abstract_data::{AbstractData, Database};
use crate::table::album::AlbumCombined;
use crate::table::database::{DatabaseSchema, MediaCombined};
use std::sync::{Arc, LazyLock, RwLock, atomic::AtomicU64};

pub struct Tree {
    pub in_disk: Pool<SqliteConnectionManager>,
    pub in_memory: &'static Arc<RwLock<Vec<AbstractData>>>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree::new());

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl Tree {
    pub fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        let conn = self.in_disk.get().context("Failed to get DB connection")?;
        Ok(conn)
    }
    pub fn load_from_db(&self, id: &str) -> Result<AbstractData> {
        let conn = self.get_connection()?;

        // 嘗試從 object 表查詢類型
        if let Ok(obj_type) =
            conn.query_row("SELECT obj_type FROM object WHERE id = ?", [id], |row| {
                row.get::<_, String>(0)
            })
        {
            match obj_type.as_str() {
                "album" => {
                    // 讀取相簿
                    let album = AlbumCombined::get_all(&conn)?
                        .into_iter()
                        .find(|a| a.object.id.as_str() == id)
                        .ok_or_else(|| anyhow::anyhow!("Album not found"))?;
                    Ok(AbstractData::Album(album))
                }
                "image" | "video" => {
                    // 讀取媒體
                    let media = MediaCombined::get_by_id(&conn, id)?;
                    Ok(AbstractData::Media(media))
                }
                _ => Err(anyhow::anyhow!("Unknown object type")),
            }
        } else {
            Err(anyhow::anyhow!("No data found for id: {}", id))
        }
    }

    // 修改回傳型別為 Vec<Database>，因為單純的 Schema 在這裡可能不夠用
    pub fn load_all_databases_from_db(&self) -> Result<Vec<Database>> {
        let conn = self.get_connection()?;

        // 使用 MediaCombined::get_all() 來載入所有媒體
        let all_media = MediaCombined::get_all(&conn)?;

        let mut databases = Vec::new();

        for media in all_media {
            // 將 MediaCombined 轉換為舊的 Database 格式以保持兼容性
            let (schema, album_set) = self.media_combined_to_database(&conn, media)?;
            databases.push(Database {
                schema,
                album: album_set,
            });
        }

        Ok(databases)
    }

    pub fn load_database_from_hash(&self, hash: &str) -> Result<Database> {
        let conn = self.get_connection()?;
        
        // 1. 使用 MediaCombined 透過 ID (Hash) 獲取資料
        let media = MediaCombined::get_by_id(&conn, hash)?;

        // 2. 重用 media_combined_to_database 函式來轉換資料結構並獲取相簿關聯
        //    (此函式您已經在檔案下方定義好了)
        let (schema, album_set) = self.media_combined_to_database(&conn, media)?;

        Ok(Database {
            schema,
            album: album_set,
        })
    }

    // 輔助函數：將 MediaCombined 轉換為舊的 Database 格式
    fn media_combined_to_database(
        &self,
        conn: &rusqlite::Connection,
        media: MediaCombined,
    ) -> Result<(
        crate::table::database::DatabaseSchema,
        HashSet<ArrayString<64>>,
    )> {
        use crate::table::database::DatabaseSchema;

        let (hash, size, width, height, thumbhash, phash, ext, ext_type, pending, timestamp_ms) =
            match media {
                MediaCombined::Image(img) => (
                    img.object.id,
                    img.metadata.size,
                    img.metadata.width,
                    img.metadata.height,
                    img.object.thumbhash.unwrap_or_default(),
                    img.metadata.phash.unwrap_or_default(),
                    img.metadata.ext,
                    "image".to_string(),
                    img.object.pending,
                    img.object.created_time,
                ),
                MediaCombined::Video(vid) => (
                    vid.object.id,
                    vid.metadata.size,
                    vid.metadata.width,
                    vid.metadata.height,
                    vid.object.thumbhash.unwrap_or_default(),
                    Vec::new(), // Video 沒有 phash
                    vid.metadata.ext,
                    "video".to_string(),
                    vid.object.pending,
                    vid.object.created_time,
                ),
            };

        let schema = DatabaseSchema {
            hash,
            size,
            width,
            height,
            thumbhash,
            phash,
            ext,
            ext_type,
            pending,
            timestamp_ms,
        };

        // 讀取相簿關聯
        let mut stmt_albums =
            conn.prepare("SELECT album_id FROM album_databases WHERE hash = ?")?;
        let albums =
            stmt_albums.query_map([schema.hash.as_str()], |row| row.get::<_, String>(0))?;
        let mut album_set = HashSet::new();
        for album_id in albums {
            if let Ok(as_str) = ArrayString::from(&album_id?) {
                album_set.insert(as_str);
            }
        }

        Ok((schema, album_set))
    }
}
