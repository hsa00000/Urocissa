use crate::table::object::Object;
use rusqlite::Connection;
use sea_query::{ColumnDef, ForeignKey, Iden, Index, SqliteQueryBuilder, Table};
use serde::{Deserialize, Serialize};

#[derive(Iden)]
pub enum DatabaseExif {
    Table, // "database_exif"
    Hash,
    Tag,
    Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExifSchema {
    pub hash: String,
    pub tag: String,
    pub value: String,
}

pub struct DatabaseExifTable;

impl DatabaseExifTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = Table::create()
            .table(DatabaseExif::Table)
            .if_not_exists()
            .col(ColumnDef::new(DatabaseExif::Hash).text().not_null())
            .col(ColumnDef::new(DatabaseExif::Tag).text().not_null())
            .col(ColumnDef::new(DatabaseExif::Value).text().not_null())
            .primary_key(
                Index::create()
                    .col(DatabaseExif::Hash)
                    .col(DatabaseExif::Tag),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(DatabaseExif::Table, DatabaseExif::Hash)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);
        conn.execute(&sql, [])?;

        let idx = Index::create()
            .if_not_exists()
            .name("idx_database_exif_tag")
            .table(DatabaseExif::Table)
            .col(DatabaseExif::Tag)
            .to_string(SqliteQueryBuilder);
        conn.execute(&idx, [])?;
        Ok(())
    }
}
