pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OptionalExtension;

use crate::public::structure::abstract_data::AbstractData;
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

    pub fn load_from_db(&self, id: impl AsRef<str>) -> Result<AbstractData> {
        let id = id.as_ref();
        let conn = self.get_connection()?;

        // 嘗試從 object 表查詢類型
        if let Ok(obj_type) =
            conn.query_row("SELECT obj_type FROM object WHERE id = ?", [id], |row| {
                row.get::<_, String>(0)
            })
        {
            match obj_type.as_str() {
                "album" => {
                    let album = AlbumCombined::get_by_id(&conn, id)?;
                    Ok(AbstractData::Album(album))
                }
                "image" => {
                    let image = ImageCombined::get_by_id(&conn, id)?;
                    Ok(AbstractData::Image(image))
                }
                "video" => {
                    let video = VideoCombined::get_by_id(&conn, id)?;
                    Ok(AbstractData::Video(video))
                }
                _ => Err(anyhow::anyhow!("Unknown object type")),
            }
        } else {
            Err(anyhow::anyhow!("No data found for id: {}", id))
        }
    }

    // 回傳型別改為 Vec<AbstractData>
    pub fn load_all_data_from_db(&self) -> Result<Vec<AbstractData>> {
        let conn = self.get_connection()?;

        let all_images = ImageCombined::get_all(&conn)?;
        let all_videos = VideoCombined::get_all(&conn)?;

        let mut result = Vec::with_capacity(all_images.len() + all_videos.len());

        for image in all_images {
            result.push(AbstractData::Image(image));
        }

        for video in all_videos {
            result.push(AbstractData::Video(video));
        }

        Ok(result)
    }

    pub fn load_data_from_hash(&self, hash: impl AsRef<str>) -> Result<Option<AbstractData>> {
        let hash = hash.as_ref();
        let conn = self.get_connection()?;

        let type_sql = "SELECT obj_type FROM object WHERE id = ?";
        let obj_type: Option<String> = conn
            .query_row(type_sql, [hash], |row| row.get(0))
            .optional()?;

        if let Some(obj_type) = obj_type {
            match obj_type.as_str() {
                "image" => {
                    let image = ImageCombined::get_by_id(&conn, hash)?;
                    Ok(Some(AbstractData::Image(image)))
                }
                "video" => {
                    let video = VideoCombined::get_by_id(&conn, hash)?;
                    Ok(Some(AbstractData::Video(video)))
                }
                "album" => {
                    let album = AlbumCombined::get_by_id(&conn, hash)?;
                    Ok(Some(AbstractData::Album(album)))
                }
                _ => Err(anyhow::anyhow!("Unknown object type for hash: {}", hash)),
            }
        } else {
            Ok(None)
        }
    }
}
