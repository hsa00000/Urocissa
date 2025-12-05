use crate::table::object::Object;
use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use sea_query::{ColumnDef, ForeignKey, Iden, SqliteQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Iden)]
pub enum MetaAlbum {
    Table,
    Id,
    Title,
    StartTime,
    EndTime,
    LastModifiedTime,
    Cover,
    UserDefinedMetadata,
    ItemCount,
    ItemSize,
}

/// AlbumMetadataSchema: 相簿專用屬性
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumMetadataSchema {
    pub id: ArrayString<64>,
    pub title: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub last_modified_time: i64,
    pub cover: Option<ArrayString<64>>,
    pub user_defined_metadata: HashMap<String, Vec<String>>,
    pub item_count: usize,
    pub item_size: u64,
}

impl AlbumMetadataSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = Table::create()
            .table(MetaAlbum::Table)
            .if_not_exists()
            .col(ColumnDef::new(MetaAlbum::Id).text().primary_key())
            .col(ColumnDef::new(MetaAlbum::Title).text())
            .col(ColumnDef::new(MetaAlbum::StartTime).integer())
            .col(ColumnDef::new(MetaAlbum::EndTime).integer())
            .col(ColumnDef::new(MetaAlbum::LastModifiedTime).integer())
            .col(ColumnDef::new(MetaAlbum::Cover).text())
            .col(ColumnDef::new(MetaAlbum::UserDefinedMetadata).text())
            .col(
                ColumnDef::new(MetaAlbum::ItemCount)
                    .integer()
                    .default(0),
            )
            .col(
                ColumnDef::new(MetaAlbum::ItemSize)
                    .integer()
                    .default(0),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(MetaAlbum::Table, MetaAlbum::Id)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);

        conn.execute(&sql, [])?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id_str: String = row.get(MetaAlbum::Id.to_string().as_str())?;
        let title: Option<String> = row.get(MetaAlbum::Title.to_string().as_str())?;
        let start_time: Option<i64> = row.get(MetaAlbum::StartTime.to_string().as_str())?;
        let end_time: Option<i64> = row.get(MetaAlbum::EndTime.to_string().as_str())?;
        let last_modified_time: i64 = row.get(MetaAlbum::LastModifiedTime.to_string().as_str())?;

        let cover_str: Option<String> = row.get(MetaAlbum::Cover.to_string().as_str())?;
        let cover: Option<ArrayString<64>> = cover_str.and_then(|s| ArrayString::from(&s).ok());

        let user_defined_metadata_str: String =
            row.get(MetaAlbum::UserDefinedMetadata.to_string().as_str())?;
        let user_defined_metadata: HashMap<String, Vec<String>> =
            serde_json::from_str(&user_defined_metadata_str).unwrap_or_default();
        let item_count: usize =
            row.get::<_, i64>(MetaAlbum::ItemCount.to_string().as_str())? as usize;
        let item_size: u64 = row.get(MetaAlbum::ItemSize.to_string().as_str())?;

        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            title,
            start_time,
            end_time,
            last_modified_time,
            cover,
            user_defined_metadata,
            item_count,
            item_size,
        })
    }

    pub fn new(id: ArrayString<64>, title: Option<String>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        Self {
            id,
            title,
            start_time: None,
            end_time: None,
            last_modified_time: timestamp,
            cover: None,
            user_defined_metadata: HashMap::new(),
            item_count: 0,
            item_size: 0,
        }
    }
}
