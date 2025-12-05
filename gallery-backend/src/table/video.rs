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
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

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
    /// 根據 Hash (ID) 讀取單一影片資料
    pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        let (sql, values) = Query::select()
            .columns([
                (Object::Table, Object::Id),
                (Object::Table, Object::ObjType),
                (Object::Table, Object::CreatedTime),
                (Object::Table, Object::Pending),
                (Object::Table, Object::Thumbhash),
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
            .and_where(Expr::col((Object::Table, Object::Id)).eq(id).into())
            .build_rusqlite(SqliteQueryBuilder);

        let mut video = conn.query_row(&sql, &*values.as_params(), Self::from_row)?;

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
                    video.albums.insert(as_str);
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
                video.object.tags.insert(t);
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
                video.exif_vec.insert(k, v);
            }
        }

        Ok(video)
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

        // 2. 批次讀取所有「影片」類型的相簿關聯
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
                    .eq("video")
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

        // 3. 批次讀取所有「影片」類型的標籤關聯
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
                    .eq("video")
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

        // 4. 批次讀取所有「影片」類型的 EXIF
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
                    .eq("video")
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

        // 5. 將資料填回影片 Struct
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

    pub fn imported_path(&self) -> PathBuf {
        PathBuf::from(self.imported_path_string())
    }

    pub fn imported_path_string(&self) -> String {
        format!(
            "./object/imported/{}/{}.{}",
            &self.object.id[0..2],
            self.object.id,
            self.metadata.ext
        )
    }
}
