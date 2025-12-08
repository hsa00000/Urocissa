use crate::table::meta_image::MetaImage;
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

use crate::table::meta_image::ImageMetadataSchema;
use crate::table::object::ObjectSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: ImageMetadataSchema,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
}

impl ImageCombined {
    /// 根據 Hash (ID) 讀取單一圖片資料（包含所屬相簿、標籤與 EXIF）
    pub fn get_by_id(conn: &Connection, id: impl AsRef<str>) -> rusqlite::Result<Self> {
        let id = id.as_ref();

        // 1. 讀取本體
        let mut image = Self::fetch_basic_info(conn, id)?;

        // 2. 呼叫共用邏輯讀取關聯
        image.albums = AlbumDatabase::fetch_albums(conn, id)?;
        image.object.tags = TagDatabase::fetch_tags(conn, id)?;
        image.exif_vec = DatabaseExif::fetch_exif(conn, id)?;

        Ok(image)
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
                (MetaImage::Table, MetaImage::Size),
                (MetaImage::Table, MetaImage::Width),
                (MetaImage::Table, MetaImage::Height),
                (MetaImage::Table, MetaImage::Ext),
                (MetaImage::Table, MetaImage::Phash),
            ])
            .from(Object::Table)
            .join(
                JoinType::InnerJoin,
                MetaImage::Table,
                Expr::col((Object::Table, Object::Id)).equals((MetaImage::Table, MetaImage::Id)),
            )
            .and_where(Expr::col((Object::Table, Object::Id)).eq(id))
            .build_rusqlite(SqliteQueryBuilder);

        conn.query_row(&sql, &*values.as_params(), Self::from_row)
    }

    /// 讀取所有圖片資料（高效能批次填入相簿、標籤與 EXIF 關聯）
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        // 1. 讀取所有圖片本體
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
                (MetaImage::Table, MetaImage::Size),
                (MetaImage::Table, MetaImage::Width),
                (MetaImage::Table, MetaImage::Height),
                (MetaImage::Table, MetaImage::Ext),
                (MetaImage::Table, MetaImage::Phash),
            ])
            .from(Object::Table)
            .join(
                JoinType::InnerJoin,
                MetaImage::Table,
                Expr::col((Object::Table, Object::Id)).equals((MetaImage::Table, MetaImage::Id)),
            )
            .and_where(Expr::col((Object::Table, Object::ObjType)).eq("image"))
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), Self::from_row)?;

        let mut images: Vec<Self> = rows.collect::<Result<_, _>>()?;

        if images.is_empty() {
            return Ok(images);
        }

        // 2. 批次讀取所有關聯資料 (只用 3 個 SQL)
        let mut album_map = AlbumDatabase::fetch_all_albums(conn, "image")?;
        let mut tag_map = TagDatabase::fetch_all_tags(conn, "image")?;
        let mut exif_map = DatabaseExif::fetch_all_exif(conn, "image")?;

        // 3. 將資料填回圖片 Struct (記憶體操作，速度快)
        for image in &mut images {
            if let Some(albums) = album_map.remove(&image.object.id) {
                image.albums = albums;
            }
            if let Some(tags) = tag_map.remove(&image.object.id) {
                image.object.tags = tags;
            }
            if let Some(exif) = exif_map.remove(&image.object.id) {
                image.exif_vec = exif;
            }
        }
        for image in &mut images {
            if let Some(albums) = album_map.remove(&image.object.id) {
                image.albums = albums;
            }
            if let Some(tags) = tag_map.remove(&image.object.id) {
                image.object.tags = tags;
            }
            if let Some(exif) = exif_map.remove(&image.object.id) {
                image.exif_vec = exif;
            }
        }

        Ok(images)
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(ImageCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: ImageMetadataSchema::from_row(row)?,
            albums: HashSet::new(),
            exif_vec: BTreeMap::new(),
        })
    }
}
