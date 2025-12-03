pub mod new;
pub mod read_tags;

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OptionalExtension;
use std::collections::HashSet;

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
                    let album = AlbumCombined::get_all(&conn)?
                        .into_iter()
                        .find(|a| a.object.id.as_str() == id)
                        .ok_or_else(|| anyhow::anyhow!("Album not found"))?;
                    Ok(AbstractData::Album(album))
                }
                "image" => {
                    let mut image = ImageCombined::get_by_id(&conn, id)?;
                    // 填入 albums
                    image.albums = self.get_album_associations(&conn, &image.object.id)?;
                    Ok(AbstractData::Image(image))
                }
                "video" => {
                    let mut video = VideoCombined::get_by_id(&conn, id)?;
                    // 填入 albums
                    video.albums = self.get_album_associations(&conn, &video.object.id)?;
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

        // 一次性讀取所有 album 關聯
        let mut stmt = conn.prepare("SELECT hash, album_id FROM album_database")?;
        let relations: Vec<(String, String)> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;

        let mut relation_map: std::collections::HashMap<String, HashSet<ArrayString<64>>> = std::collections::HashMap::new();
        for (hash, album_id) in relations {
            if let Ok(as_str) = ArrayString::from(&album_id) {
                relation_map.entry(hash).or_insert_with(HashSet::new).insert(as_str);
            }
        }

        let mut result = Vec::with_capacity(all_images.len() + all_videos.len());

        for mut image in all_images {
            if let Some(albums) = relation_map.remove(image.object.id.as_str()) {
                image.albums = albums;
            }
            result.push(AbstractData::Image(image));
        }
        
        for mut video in all_videos {
            if let Some(albums) = relation_map.remove(video.object.id.as_str()) {
                video.albums = albums;
            }
            result.push(AbstractData::Video(video));
        }

        Ok(result)
    }

    pub fn load_data_from_hash(&self, hash: &str) -> Result<Option<AbstractData>> {
        let conn = self.get_connection()?;

        let type_sql = "SELECT obj_type FROM object WHERE id = ?";
        let obj_type: Option<String> = conn
            .query_row(type_sql, [hash], |row| row.get(0))
            .optional()?;

        if let Some(obj_type) = obj_type {
            match obj_type.as_str() {
                "image" => {
                    let mut image = ImageCombined::get_by_id(&conn, hash)?;
                    image.albums = self.get_album_associations(&conn, &image.object.id)?;
                    Ok(Some(AbstractData::Image(image)))
                }
                "video" => {
                    let mut video = VideoCombined::get_by_id(&conn, hash)?;
                    video.albums = self.get_album_associations(&conn, &video.object.id)?;
                    Ok(Some(AbstractData::Video(video)))
                }
                "album" => {
                     let album = AlbumCombined::get_all(&conn)?
                        .into_iter()
                        .find(|a| a.object.id.as_str() == hash)
                        .ok_or_else(|| anyhow::anyhow!("Album not found"))?;
                     Ok(Some(AbstractData::Album(album)))
                }
                _ => Err(anyhow::anyhow!("Unknown object type for hash: {}", hash)),
            }
        } else {
            Ok(None)
        }
    }

    fn get_album_associations(
        &self,
        conn: &rusqlite::Connection,
        hash: &ArrayString<64>,
    ) -> Result<HashSet<ArrayString<64>>> {
        let mut stmt_albums = conn.prepare("SELECT album_id FROM album_database WHERE hash = ?")?;
        let albums = stmt_albums.query_map([hash.as_str()], |row| row.get::<_, String>(0))?;
        let mut album_set = HashSet::new();
        for album_id in albums {
            if let Ok(as_str) = ArrayString::from(&album_id?) {
                album_set.insert(as_str);
            }
        }
        Ok(album_set)
    }
}
