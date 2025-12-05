use crate::table::object::Object;
use rusqlite::Connection;
use sea_query::{ColumnDef, ForeignKey, Iden, Index, SqliteQueryBuilder, Table};
use serde::{Deserialize, Serialize};

#[derive(Iden)]
pub enum TagDatabase {
    Table, // "tag_database"
    Hash,
    Tag,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TagDatabaseSchema {
    pub hash: String,
    pub tag: String,
}

pub struct TagDatabasesTable;

impl TagDatabasesTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        // 1. Create Table
        let sql = Table::create()
            .table(TagDatabase::Table)
            .if_not_exists()
            .col(ColumnDef::new(TagDatabase::Hash).text().not_null())
            .col(ColumnDef::new(TagDatabase::Tag).text().not_null())
            .primary_key(
                Index::create()
                    .col(TagDatabase::Hash)
                    .col(TagDatabase::Tag),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(TagDatabase::Table, TagDatabase::Hash)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);

        conn.execute(&sql, [])?;

        // 2. Create Index
        let idx = Index::create()
            .if_not_exists()
            .name("idx_tag_databases_tag")
            .table(TagDatabase::Table)
            .col(TagDatabase::Tag)
            .to_string(SqliteQueryBuilder);
        
        conn.execute(&idx, [])?;
        Ok(())
    }
}
