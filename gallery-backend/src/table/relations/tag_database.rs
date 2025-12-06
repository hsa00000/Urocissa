use crate::table::object::Object;
use arrayvec::ArrayString;
use rusqlite::Connection;
use sea_query::{
    ColumnDef, Expr, ExprTrait, ForeignKey, Iden, Index, JoinType, Query, SqliteQueryBuilder, Table,
};
use sea_query_rusqlite::RusqliteBinder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
            .primary_key(Index::create().col(TagDatabase::Hash).col(TagDatabase::Tag))
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

impl TagDatabase {
    /// 通用方法：根據 Hash (ID) 取得所有標籤
    pub fn fetch_tags(conn: &Connection, id: &str) -> rusqlite::Result<HashSet<String>> {
        let (sql, values) = Query::select()
            .column(TagDatabase::Tag)
            .from(TagDatabase::Table)
            .and_where(Expr::col(TagDatabase::Hash).eq(id))
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), |row| row.get::<_, String>(0))?;

        let mut tags = HashSet::new();
        for tag in rows {
            if let Ok(t) = tag {
                tags.insert(t);
            }
        }
        Ok(tags)
    }

    /// 通用方法：批次取得某種類型物件的所有標籤 (解決 N+1 問題)
    pub fn fetch_all_tags(
        conn: &Connection,
        obj_type: &str,
    ) -> rusqlite::Result<HashMap<ArrayString<64>, HashSet<String>>> {
        let (sql, values) = Query::select()
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
            .and_where(Expr::col((Object::Table, Object::ObjType)).eq(obj_type))
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&*values.as_params(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut map: HashMap<ArrayString<64>, HashSet<String>> = HashMap::new();

        for row in rows {
            let (hash, tag) = row?;
            if let Ok(hash_as) = ArrayString::from(&hash) {
                map.entry(hash_as).or_default().insert(tag);
            }
        }
        Ok(map)
    }
}
