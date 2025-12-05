use crate::public::db::tree::TREE;
use crate::table::meta_album::MetaAlbum;
use arrayvec::ArrayString;
use rusqlite::Connection;
use sea_query::{
    ColumnDef, Expr, ExprTrait, ForeignKey, Iden, Index, JoinType, Query, SqliteQueryBuilder, Table,
};
use sea_query_rusqlite::RusqliteBinder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Iden)]
pub enum AlbumShare {
    Table, // "album_share"
    AlbumId,
    Url,
    Description,
    Password,
    ShowMetadata,
    ShowDownload,
    ShowUpload,
    Exp,
}

/// Share: 用於前端傳輸的分享結構
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub url: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: u64,
}

/// ResolvedShare: 包含 album 資訊的完整分享結構
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedShare {
    #[serde(flatten)]
    pub share: Share,
    pub album_id: ArrayString<64>,
    pub album_title: Option<String>,
}

impl ResolvedShare {
    pub fn new(album_id: ArrayString<64>, album_title: Option<String>, share: Share) -> Self {
        Self {
            share,
            album_id,
            album_title,
        }
    }
}

pub struct AlbumShareTable;

impl AlbumShareTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = Table::create()
            .table(AlbumShare::Table)
            .if_not_exists()
            .col(ColumnDef::new(AlbumShare::AlbumId).text().not_null())
            .col(ColumnDef::new(AlbumShare::Url).text().not_null())
            .col(ColumnDef::new(AlbumShare::Description).text().not_null())
            .col(ColumnDef::new(AlbumShare::Password).text())
            .col(
                ColumnDef::new(AlbumShare::ShowMetadata)
                    .integer()
                    .not_null(),
            )
            .col(
                ColumnDef::new(AlbumShare::ShowDownload)
                    .integer()
                    .not_null(),
            )
            .col(ColumnDef::new(AlbumShare::ShowUpload).integer().not_null())
            .col(ColumnDef::new(AlbumShare::Exp).integer().not_null())
            .primary_key(
                Index::create()
                    .col(AlbumShare::AlbumId)
                    .col(AlbumShare::Url),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AlbumShare::Table, AlbumShare::AlbumId)
                    .to(MetaAlbum::Table, MetaAlbum::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);
        conn.execute(&sql, [])?;

        let idx = Index::create()
            .if_not_exists()
            .name("idx_album_share_url")
            .table(AlbumShare::Table)
            .col(AlbumShare::Url)
            .to_string(SqliteQueryBuilder);
        conn.execute(&idx, [])?;

        Ok(())
    }

    pub fn get_all_shares_grouped()
    -> rusqlite::Result<HashMap<String, HashMap<ArrayString<64>, Share>>> {
        let conn = TREE.get_connection().unwrap();

        // SELECT * FROM album_share
        let (sql, values) = Query::select()
            .columns([
                AlbumShare::AlbumId,
                AlbumShare::Url,
                AlbumShare::Description,
                AlbumShare::Password,
                AlbumShare::ShowMetadata,
                AlbumShare::ShowDownload,
                AlbumShare::ShowUpload,
                AlbumShare::Exp,
            ])
            .from(AlbumShare::Table)
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let share_iter = stmt.query_map(&*values.as_params(), |row| {
            let album_id: String = row.get(AlbumShare::AlbumId.to_string().as_str())?;
            let url_str: String = row.get(AlbumShare::Url.to_string().as_str())?;
            let url = ArrayString::from(&url_str).unwrap();

            Ok((
                album_id,
                url,
                Share {
                    url,
                    description: row.get(AlbumShare::Description.to_string().as_str())?,
                    password: row.get(AlbumShare::Password.to_string().as_str())?,
                    show_metadata: row.get(AlbumShare::ShowMetadata.to_string().as_str())?,
                    show_download: row.get(AlbumShare::ShowDownload.to_string().as_str())?,
                    show_upload: row.get(AlbumShare::ShowUpload.to_string().as_str())?,
                    exp: row.get(AlbumShare::Exp.to_string().as_str())?,
                },
            ))
        })?;

        let mut map: HashMap<String, HashMap<ArrayString<64>, Share>> = HashMap::new();

        for share_result in share_iter {
            if let Ok((album_id, url, share)) = share_result {
                map.entry(album_id).or_default().insert(url, share);
            }
        }

        Ok(map)
    }

    pub fn get_all_resolved() -> rusqlite::Result<Vec<ResolvedShare>> {
        let conn = TREE.get_connection().unwrap();

        // SELECT ... FROM album_share LEFT JOIN meta_album ...
        let (sql, values) = Query::select()
            .columns([
                (AlbumShare::Table, AlbumShare::Url),
                (AlbumShare::Table, AlbumShare::Description),
                (AlbumShare::Table, AlbumShare::Password),
                (AlbumShare::Table, AlbumShare::ShowMetadata),
                (AlbumShare::Table, AlbumShare::ShowDownload),
                (AlbumShare::Table, AlbumShare::ShowUpload),
                (AlbumShare::Table, AlbumShare::Exp),
                (AlbumShare::Table, AlbumShare::AlbumId),
            ])
            .column((MetaAlbum::Table, MetaAlbum::Title))
            .from(AlbumShare::Table)
            .join(
                JoinType::LeftJoin,
                MetaAlbum::Table,
                Expr::col((AlbumShare::Table, AlbumShare::AlbumId))
                    .equals((MetaAlbum::Table, MetaAlbum::Id)),
            )
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let share_iter = stmt.query_map(&*values.as_params(), |row| {
            let url_str: String = row.get(AlbumShare::Url.to_string().as_str())?;
            let url = ArrayString::from(&url_str).unwrap();

            let album_id_str: String = row.get(AlbumShare::AlbumId.to_string().as_str())?;
            let album_id = ArrayString::from(&album_id_str).unwrap();

            Ok(ResolvedShare {
                share: Share {
                    url,
                    description: row.get(AlbumShare::Description.to_string().as_str())?,
                    password: row.get(AlbumShare::Password.to_string().as_str())?,
                    show_metadata: row.get(AlbumShare::ShowMetadata.to_string().as_str())?,
                    show_download: row.get(AlbumShare::ShowDownload.to_string().as_str())?,
                    show_upload: row.get(AlbumShare::ShowUpload.to_string().as_str())?,
                    exp: row.get(AlbumShare::Exp.to_string().as_str())?,
                },
                album_id,
                album_title: row.get(MetaAlbum::Title.to_string().as_str())?,
            })
        })?;

        let mut shares = Vec::new();
        for share in share_iter {
            if let Ok(s) = share {
                shares.push(s);
            }
        }
        Ok(shares)
    }
}
