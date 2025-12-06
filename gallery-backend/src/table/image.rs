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
use std::collections::{BTreeMap, HashMap, HashSet};

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
        // 1. 讀取本體資料
        let (sql, values) = Query::select()
            .columns([
                (Object::Table, Object::Id),
                (Object::Table, Object::ObjType),
                (Object::Table, Object::CreatedTime),
                (Object::Table, Object::Pending),
                (Object::Table, Object::Thumbhash),
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
            .and_where(Expr::col((Object::Table, Object::Id)).eq(id).into())
            .build_rusqlite(SqliteQueryBuilder);

        let mut image = conn.query_row(&sql, &*values.as_params(), Self::from_row)?;

        // 2. 讀取關聯相簿
        let (sql_albums, values_albums) = Query::select()
            .column(AlbumDatabase::AlbumId)
            .from(AlbumDatabase::Table)
            .and_where(Expr::col(AlbumDatabase::Hash).eq(id).into())
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt_albums = conn.prepare(&sql_albums)?;
        let album_rows =
            stmt_albums.query_map(&*values_albums.as_params(), |row| row.get::<_, String>(0))?;

        for album_id in album_rows {
            if let Ok(id_str) = album_id {
                if let Ok(as_str) = ArrayString::from(&id_str) {
                    image.albums.insert(as_str);
                }
            }
        }

        // 3. 讀取關聯標籤
        let (sql_tags, values_tags) = Query::select()
            .column(TagDatabase::Tag)
            .from(TagDatabase::Table)
            .and_where(Expr::col(TagDatabase::Hash).eq(id).into())
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt_tags = conn.prepare(&sql_tags)?;
        let tag_rows =
            stmt_tags.query_map(&*values_tags.as_params(), |row| row.get::<_, String>(0))?;

        for tag in tag_rows {
            if let Ok(t) = tag {
                image.object.tags.insert(t);
            }
        }

        // 4. 讀取 EXIF
        let (sql_exif, values_exif) = Query::select()
            .columns([DatabaseExif::Tag, DatabaseExif::Value])
            .from(DatabaseExif::Table)
            .and_where(Expr::col(DatabaseExif::Hash).eq(id).into())
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt_exif = conn.prepare(&sql_exif)?;
        let exif_rows = stmt_exif.query_map(&*values_exif.as_params(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in exif_rows {
            if let Ok((k, v)) = row {
                image.exif_vec.insert(k, v);
            }
        }

        Ok(image)
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
            .and_where(
                Expr::col((Object::Table, Object::ObjType))
                    .eq("image")
                    .into(),
            )
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), Self::from_row)?;

        let mut images: Vec<Self> = rows.collect::<Result<_, _>>()?;

        if images.is_empty() {
            return Ok(images);
        }

        // 2. 批次讀取所有「圖片」類型的相簿關聯
        let (sql_album_relations, values_album_rel) = Query::select()
            .columns([
                (AlbumDatabase::Table, AlbumDatabase::Hash),
                (AlbumDatabase::Table, AlbumDatabase::AlbumId),
            ])
            .from(AlbumDatabase::Table)
            .join(
                JoinType::InnerJoin,
                Object::Table,
                Expr::col((AlbumDatabase::Table, AlbumDatabase::Hash))
                    .equals((Object::Table, Object::Id)),
            )
            .and_where(
                Expr::col((Object::Table, Object::ObjType))
                    .eq("image")
                    .into(),
            )
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt_album_rel = conn.prepare(&sql_album_relations)?;
        let album_rel_rows = stmt_album_rel.query_map(&*values_album_rel.as_params(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut album_map: HashMap<ArrayString<64>, HashSet<ArrayString<64>>> = HashMap::new();

        for rel in album_rel_rows {
            let (hash, album_id) = rel?;
            if let (Ok(hash_as), Ok(album_as)) =
                (ArrayString::from(&hash), ArrayString::from(&album_id))
            {
                album_map.entry(hash_as).or_default().insert(album_as);
            }
        }

        // 3. 批次讀取所有「圖片」類型的標籤關聯
        let (sql_tag_relations, values_tag_rel) = Query::select()
            .columns([
                (TagDatabase::Table, TagDatabase::Hash),
                (TagDatabase::Table, TagDatabase::Tag),
            ])
            .from(TagDatabase::Table)
            .join(
                JoinType::InnerJoin,
                Object::Table,
                Expr::col((TagDatabase::Table, TagDatabase::Hash))
                    .equals((Object::Table, Object::Id)),
            )
            .and_where(
                Expr::col((Object::Table, Object::ObjType))
                    .eq("image")
                    .into(),
            )
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt_tag_rel = conn.prepare(&sql_tag_relations)?;
        let tag_rel_rows = stmt_tag_rel.query_map(&*values_tag_rel.as_params(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut tag_map: HashMap<ArrayString<64>, HashSet<String>> = HashMap::new();

        for rel in tag_rel_rows {
            let (hash, tag) = rel?;
            if let Ok(hash_as) = ArrayString::from(&hash) {
                tag_map.entry(hash_as).or_default().insert(tag);
            }
        }

        // 4. 批次讀取所有「圖片」類型的 EXIF
        let (sql_exif_relations, values_exif_rel) = Query::select()
            .columns([
                (DatabaseExif::Table, DatabaseExif::Hash),
                (DatabaseExif::Table, DatabaseExif::Tag),
                (DatabaseExif::Table, DatabaseExif::Value),
            ])
            .from(DatabaseExif::Table)
            .join(
                JoinType::InnerJoin,
                Object::Table,
                Expr::col((DatabaseExif::Table, DatabaseExif::Hash))
                    .equals((Object::Table, Object::Id)),
            )
            .and_where(
                Expr::col((Object::Table, Object::ObjType))
                    .eq("image")
                    .into(),
            )
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt_exif_rel = conn.prepare(&sql_exif_relations)?;
        let exif_rel_rows = stmt_exif_rel.query_map(&*values_exif_rel.as_params(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut exif_map: HashMap<ArrayString<64>, BTreeMap<String, String>> = HashMap::new();

        for rel in exif_rel_rows {
            let (hash, k, v) = rel?;
            if let Ok(hash_as) = ArrayString::from(&hash) {
                exif_map.entry(hash_as).or_default().insert(k, v);
            }
        }

        // 5. 將資料填回圖片 Struct
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
