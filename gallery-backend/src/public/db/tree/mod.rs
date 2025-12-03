pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::{HashMap, HashSet};

use crate::public::structure::abstract_data::{AbstractData, Database, MediaWithAlbum};
use crate::table::album::AlbumCombined;
use crate::table::image::ImageCombined;
use crate::table::video::VideoCombined;
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
                "image" => {
                    // 讀取圖片
                    let image = ImageCombined::get_by_id(&conn, id)?;
                    Ok(AbstractData::Image(image))
                }
                "video" => {
                    // 讀取影片
                    let video = VideoCombined::get_by_id(&conn, id)?;
                    Ok(AbstractData::Video(video))
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

        // 載入所有圖片
        let all_images = ImageCombined::get_all(&conn)?;
        // 載入所有影片
        let all_videos = VideoCombined::get_all(&conn)?;

        let mut databases = Vec::new();

        for image in all_images {
            // 將 ImageCombined 轉換為舊的 Database 格式以保持兼容性
            let (media, album_set) = self.image_combined_to_database(&conn, image)?;
            databases.push(Database {
                media,
                album: album_set,
            });
        }

        for video in all_videos {
            // 將 VideoCombined 轉換為舊的 Database 格式以保持兼容性
            let (media, album_set) = self.video_combined_to_database(&conn, video)?;
            databases.push(Database {
                media,
                album: album_set,
            });
        }

        Ok(databases)
    }

    pub fn load_database_from_hash(&self, hash: &str) -> Result<Database> {
        let conn = self.get_connection()?;

        // 1. 先查詢 object 表確認類型
        let type_sql = "SELECT obj_type FROM object WHERE id = ?";
        let obj_type: String = conn.query_row(type_sql, [hash], |row| row.get(0))?;

        // 2. 根據類型載入資料並轉換
        match obj_type.as_str() {
            "image" => {
                let image = ImageCombined::get_by_id(&conn, hash)?;
                let (media, album_set) = self.image_combined_to_database(&conn, image)?;
                Ok(Database {
                    media,
                    album: album_set,
                })
            }
            "video" => {
                let video = VideoCombined::get_by_id(&conn, hash)?;
                let (media, album_set) = self.video_combined_to_database(&conn, video)?;
                Ok(Database {
                    media,
                    album: album_set,
                })
            }
            _ => Err(anyhow::anyhow!("Unknown object type for hash: {}", hash)),
        }
    }

    // 輔助函數：將 MediaCombined 轉換為舊的 Database 格式
    fn image_combined_to_database(
        &self,
        conn: &rusqlite::Connection,
        image: ImageCombined,
    ) -> Result<(
        MediaWithAlbum,
        HashSet<ArrayString<64>>,
    )> {
        // 讀取相簿關聯
        let album_set = self.get_album_associations(conn, &image.object.id)?;

        Ok((MediaWithAlbum::Image(image), album_set))
    }

    fn video_combined_to_database(
        &self,
        conn: &rusqlite::Connection,
        video: VideoCombined,
    ) -> Result<(
        MediaWithAlbum,
        HashSet<ArrayString<64>>,
    )> {
        // 讀取相簿關聯
        let album_set = self.get_album_associations(conn, &video.object.id)?;

        Ok((MediaWithAlbum::Video(video), album_set))
    }

    fn get_album_associations(
        &self,
        conn: &rusqlite::Connection,
        hash: &ArrayString<64>,
    ) -> Result<HashSet<ArrayString<64>>> {
        let mut stmt_albums =
            conn.prepare("SELECT album_id FROM album_databases WHERE hash = ?")?;
        let albums =
            stmt_albums.query_map([hash.as_str()], |row| row.get::<_, String>(0))?;
        let mut album_set = HashSet::new();
        for album_id in albums {
            if let Ok(as_str) = ArrayString::from(&album_id?) {
                album_set.insert(as_str);
            }
        }
        Ok(album_set)
    }
}
