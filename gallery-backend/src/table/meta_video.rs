use crate::table::object::Object;
use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use sea_query::{ColumnDef, ForeignKey, Iden, SqliteQueryBuilder, Table};
use serde::{Deserialize, Serialize};

#[derive(Iden)]
pub enum MetaVideo {
    Table,
    Id,
    Size,
    Width,
    Height,
    Ext,
    Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadataSchema {
    pub id: ArrayString<64>, // FK to object.id
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub duration: f64, // 影片時長 (秒)
}

impl VideoMetadataSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = Table::create()
            .table(MetaVideo::Table)
            .if_not_exists()
            .col(ColumnDef::new(MetaVideo::Id).text().primary_key())
            .col(ColumnDef::new(MetaVideo::Size).integer().not_null())
            .col(ColumnDef::new(MetaVideo::Width).integer().not_null())
            .col(ColumnDef::new(MetaVideo::Height).integer().not_null())
            .col(ColumnDef::new(MetaVideo::Ext).text().not_null())
            .col(
                ColumnDef::new(MetaVideo::Duration)
                    .double()
                    .default(0.0),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(MetaVideo::Table, MetaVideo::Id)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);

        conn.execute(&sql, [])?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id_str: String = row.get(MetaVideo::Id.to_string().as_str())?;
        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            size: row.get(MetaVideo::Size.to_string().as_str())?,
            width: row.get(MetaVideo::Width.to_string().as_str())?,
            height: row.get(MetaVideo::Height.to_string().as_str())?,
            ext: row.get(MetaVideo::Ext.to_string().as_str())?,
            duration: row.get(MetaVideo::Duration.to_string().as_str())?,
        })
    }

    pub fn _new(id: ArrayString<64>, size: u64, width: u32, height: u32, ext: String) -> Self {
        Self {
            id,
            size,
            width,
            height,
            ext,
            duration: 0.0,
        }
    }
}
