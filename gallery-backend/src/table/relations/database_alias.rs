use crate::table::object::Object;
use rusqlite::Connection;
use sea_query::{ColumnDef, ForeignKey, Iden, Index, SqliteQueryBuilder, Table};
use serde::{Deserialize, Serialize};

#[derive(Iden)]
pub enum DatabaseAlias {
    Table, // "database_alias"
    Hash,
    File,
    Modified,
    ScanTime,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DatabaseAliasSchema {
    pub hash: String,
    pub file: String,
    pub modified: i64,
    pub scan_time: i64,
}

pub struct DatabaseAliasTable;

impl DatabaseAliasTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = Table::create()
            .table(DatabaseAlias::Table)
            .if_not_exists()
            .col(ColumnDef::new(DatabaseAlias::Hash).text().not_null())
            .col(ColumnDef::new(DatabaseAlias::File).text().not_null())
            .col(ColumnDef::new(DatabaseAlias::Modified).integer().not_null())
            .col(ColumnDef::new(DatabaseAlias::ScanTime).integer().not_null())
            .primary_key(
                Index::create()
                    .col(DatabaseAlias::Hash)
                    .col(DatabaseAlias::ScanTime),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(DatabaseAlias::Table, DatabaseAlias::Hash)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);
        conn.execute(&sql, [])?;

        let idx = Index::create()
            .if_not_exists()
            .name("idx_database_alias_scan_time")
            .table(DatabaseAlias::Table)
            .col(DatabaseAlias::ScanTime)
            .to_string(SqliteQueryBuilder);
        conn.execute(&idx, [])?;
        Ok(())
    }
}
