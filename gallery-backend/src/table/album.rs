use crate::table::meta_album::MetaAlbum;
use crate::table::object::Object;
use crate::table::relations::tag_database::TagDatabase;
use rusqlite::{Connection, Row};
use sea_query::{Expr, ExprTrait, JoinType, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;
use serde::{Deserialize, Serialize};

use crate::table::meta_album::AlbumMetadataSchema;
use crate::table::object::ObjectSchema;

/// 這是給 API 回傳用的組合結構，透過 serde(flatten) 保持 JSON 格式與舊版相容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: AlbumMetadataSchema,
}

impl AlbumCombined {
    /// 根據 Hash (ID) 讀取單一相簿資料
    pub fn get_by_id(conn: &Connection, id: impl AsRef<str>) -> rusqlite::Result<Self> {
        let id = id.as_ref();

        // 1. 讀取本體
        let mut album = Self::fetch_basic_info(conn, id)?;

        // 2. 呼叫共用邏輯 (Album 只需要 Tags)
        album.object.tags = TagDatabase::fetch_tags(conn, id)?;

        Ok(album)
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
                (MetaAlbum::Table, MetaAlbum::Title),
                (MetaAlbum::Table, MetaAlbum::StartTime),
                (MetaAlbum::Table, MetaAlbum::EndTime),
                (MetaAlbum::Table, MetaAlbum::LastModifiedTime),
                (MetaAlbum::Table, MetaAlbum::Cover),
                (MetaAlbum::Table, MetaAlbum::UserDefinedMetadata),
                (MetaAlbum::Table, MetaAlbum::ItemCount),
                (MetaAlbum::Table, MetaAlbum::ItemSize),
            ])
            .from(Object::Table)
            .join(
                JoinType::InnerJoin,
                MetaAlbum::Table,
                Expr::col((Object::Table, Object::Id)).equals((MetaAlbum::Table, MetaAlbum::Id)),
            )
            .and_where(Expr::col((Object::Table, Object::Id)).eq(id))
            .build_rusqlite(SqliteQueryBuilder);

        conn.query_row(&sql, &*values.as_params(), Self::from_row)
    }

    /// 讀取所有相簿 (JOIN 查詢)
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        // 1. 讀取相簿本體
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
                (MetaAlbum::Table, MetaAlbum::Title),
                (MetaAlbum::Table, MetaAlbum::StartTime),
                (MetaAlbum::Table, MetaAlbum::EndTime),
                (MetaAlbum::Table, MetaAlbum::LastModifiedTime),
                (MetaAlbum::Table, MetaAlbum::Cover),
                (MetaAlbum::Table, MetaAlbum::UserDefinedMetadata),
                (MetaAlbum::Table, MetaAlbum::ItemCount),
                (MetaAlbum::Table, MetaAlbum::ItemSize),
            ])
            .from(Object::Table)
            .join(
                JoinType::InnerJoin,
                MetaAlbum::Table,
                Expr::col((Object::Table, Object::Id)).equals((MetaAlbum::Table, MetaAlbum::Id)),
            )
            .and_where(Expr::col((Object::Table, Object::ObjType)).eq("album"))
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), |row| Self::from_row(row))?;
        let mut albums: Vec<Self> = rows.collect::<rusqlite::Result<_>>()?;

        if albums.is_empty() {
            return Ok(albums);
        }

        // 2. 批次讀取相簿的標籤
        let mut tag_map = TagDatabase::fetch_all_tags(conn, "album")?;

        // 3. 將資料填回相簿 Struct
        for album in &mut albums {
            if let Some(tags) = tag_map.remove(&album.object.id) {
                album.object.tags = tags;
            }
        }

        Ok(albums)
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(AlbumCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: AlbumMetadataSchema::from_row(row)?,
        })
    }
}
