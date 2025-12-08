use crate::table::meta_video::MetaVideo;
use crate::table::object::Object;
use crate::table::relations::album_database::AlbumDatabase;
use crate::table::relations::database_exif::DatabaseExif;
use crate::table::relations::tag_database::TagDatabase;
use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use sea_query::{Expr, ExprTrait, JoinType, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use crate::table::meta_video::VideoMetadataSchema;
use crate::table::object::ObjectSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: VideoMetadataSchema,
    #[serde(default)]
    pub albums: HashSet<ArrayString<64>>,
    #[serde(default, rename = "exifVec")]
    pub exif_vec: BTreeMap<String, String>,
}

impl VideoCombined {
    pub fn get_by_id(conn: &Connection, id: impl AsRef<str>) -> rusqlite::Result<Self> {
        let id = id.as_ref();

        // 1. 讀取本體
        let mut video = Self::fetch_basic_info(conn, id)?;

        // 2. 呼叫共用邏輯讀取關聯
        video.albums = AlbumDatabase::fetch_albums(conn, id)?;
        video.object.tags = TagDatabase::fetch_tags(conn, id)?;
        video.exif_vec = DatabaseExif::fetch_exif(conn, id)?;

        Ok(video)
    }

    fn fetch_basic_info(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        let (sql, values) = Query::select()
            .columns([
                (Object::Table, Object::Id),
                (Object::Table, Object::ObjType),
                (Object::Table, Object::CreatedTime),
                (Object::Table, Object::Pending),
                (Object::Table, Object::Thumbhash),
                (Object::Table, Object::Description),
            ])
            .columns([
                (MetaVideo::Table, MetaVideo::Size),
                (MetaVideo::Table, MetaVideo::Width),
                (MetaVideo::Table, MetaVideo::Height),
                (MetaVideo::Table, MetaVideo::Ext),
                (MetaVideo::Table, MetaVideo::Duration),
            ])
            .from(Object::Table)
            .join(
                JoinType::InnerJoin,
                MetaVideo::Table,
                Expr::col((Object::Table, Object::Id)).equals((MetaVideo::Table, MetaVideo::Id)),
            )
            .and_where(Expr::col((Object::Table, Object::Id)).eq(id))
            .build_rusqlite(SqliteQueryBuilder);

        conn.query_row(&sql, &*values.as_params(), Self::from_row)
    }

    /// 讀取所有影片資料
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        // 1. 讀取所有影片本體
        let (sql, values) = Query::select()
            .columns([
                (Object::Table, Object::Id),
                (Object::Table, Object::ObjType),
                (Object::Table, Object::CreatedTime),
                (Object::Table, Object::Pending),
                (Object::Table, Object::Thumbhash),
                (Object::Table, Object::Description),
            ])
            .columns([
                (MetaVideo::Table, MetaVideo::Size),
                (MetaVideo::Table, MetaVideo::Width),
                (MetaVideo::Table, MetaVideo::Height),
                (MetaVideo::Table, MetaVideo::Ext),
                (MetaVideo::Table, MetaVideo::Duration),
            ])
            .from(Object::Table)
            .join(
                JoinType::InnerJoin,
                MetaVideo::Table,
                Expr::col((Object::Table, Object::Id)).equals((MetaVideo::Table, MetaVideo::Id)),
            )
            .and_where(
                Expr::col((Object::Table, Object::ObjType))
                    .eq("video")
                    .into(),
            )
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), Self::from_row)?;
        let mut videos: Vec<Self> = rows.collect::<Result<_, _>>()?;

        if videos.is_empty() {
            return Ok(videos);
        }

        // 2. 批次讀取所有關聯資料
        let mut album_map = AlbumDatabase::fetch_all_albums(conn, "video")?;
        let mut tag_map = TagDatabase::fetch_all_tags(conn, "video")?;
        let mut exif_map = DatabaseExif::fetch_all_exif(conn, "video")?;

        // 3. 將資料填回影片 Struct
        for video in &mut videos {
            if let Some(albums) = album_map.remove(&video.object.id) {
                video.albums = albums;
            }
            if let Some(tags) = tag_map.remove(&video.object.id) {
                video.object.tags = tags;
            }
            if let Some(exif) = exif_map.remove(&video.object.id) {
                video.exif_vec = exif;
            }
        }

        Ok(videos)
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(VideoCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: VideoMetadataSchema::from_row(row)?,
            albums: HashSet::new(),
            exif_vec: BTreeMap::new(),
        })
    }
}
