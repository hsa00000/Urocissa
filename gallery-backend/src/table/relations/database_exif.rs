use crate::table::object::Object;
use rusqlite::Connection;
use sea_query::{
    ColumnDef, Expr, ExprTrait, ForeignKey, Iden, Index, JoinType, Query, SqliteQueryBuilder, Table,
};
use sea_query_rusqlite::RusqliteBinder;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use arrayvec::ArrayString;

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

impl DatabaseExif {
    /// 通用方法：根據 Hash (ID) 取得 EXIF Map
    pub fn fetch_exif(conn: &Connection, id: &str) -> rusqlite::Result<BTreeMap<String, String>> {
        let (sql, values) = Query::select()
            .columns([DatabaseExif::Tag, DatabaseExif::Value])
            .from(DatabaseExif::Table)
            .and_where(Expr::col(DatabaseExif::Hash).eq(id))
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut exif_map = BTreeMap::new();
        for row in rows {
            if let Ok((k, v)) = row {
                exif_map.insert(k, v);
            }
        }
        Ok(exif_map)
    }

    /// 通用方法：批次取得某種類型物件的所有 EXIF
    pub fn fetch_all_exif(
        conn: &Connection,
        obj_type: &str,
    ) -> rusqlite::Result<HashMap<ArrayString<64>, BTreeMap<String, String>>> {
        let (sql, values) = Query::select()
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
            .and_where(Expr::col((Object::Table, Object::ObjType)).eq(obj_type))
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut map: HashMap<ArrayString<64>, BTreeMap<String, String>> = HashMap::new();

        for row in rows {
            let (hash, k, v) = row?;
            if let Ok(hash_as) = ArrayString::from(&hash) {
                map.entry(hash_as).or_default().insert(k, v);
            }
        }
        Ok(map)
    }
}
