use crate::table::object::Object;
use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use sea_query::{ColumnDef, ForeignKey, Iden, SqliteQueryBuilder, Table};
use serde::{Deserialize, Serialize};

#[derive(Iden)]
pub enum MetaImage {
    Table, // "meta_image"
    Id,
    Size,
    Width,
    Height,
    Ext,
    Phash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadataSchema {
    pub id: ArrayString<64>, // FK to object.id
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub phash: Option<Vec<u8>>,
}

impl ImageMetadataSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = Table::create()
            .table(MetaImage::Table)
            .if_not_exists()
            .col(ColumnDef::new(MetaImage::Id).text().primary_key())
            .col(ColumnDef::new(MetaImage::Size).integer().not_null())
            .col(ColumnDef::new(MetaImage::Width).integer().not_null())
            .col(ColumnDef::new(MetaImage::Height).integer().not_null())
            .col(ColumnDef::new(MetaImage::Ext).text().not_null())
            .col(ColumnDef::new(MetaImage::Phash).blob())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_meta_image_id")
                    .from(MetaImage::Table, MetaImage::Id)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);

        conn.execute(&sql, [])?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id_str: String = row.get(MetaImage::Id.to_string().as_str())?;
        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            size: row.get(MetaImage::Size.to_string().as_str())?,
            width: row.get(MetaImage::Width.to_string().as_str())?,
            height: row.get(MetaImage::Height.to_string().as_str())?,
            ext: row.get(MetaImage::Ext.to_string().as_str())?,
            phash: row.get(MetaImage::Phash.to_string().as_str())?,
        })
    }
}
