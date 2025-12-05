use crate::table::meta_album::MetaAlbum;
use crate::table::meta_image::MetaImage;
use crate::table::meta_video::MetaVideo;
use crate::table::object::Object;
use rusqlite::Connection;
use sea_query::{
    Asterisk, ColumnDef, Expr, ExprTrait, ForeignKey, Func, FunctionCall, Iden, Index, JoinType,
    Order, Query, SimpleExpr, SqliteQueryBuilder, Table,
};

#[derive(Iden)]
pub enum AlbumDatabase {
    Table, // "album_database"
    AlbumId,
    Hash,
}

pub struct AlbumDatabasesTable;

impl AlbumDatabasesTable {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        // 1. 使用 SeaQuery 建立 Table
        let table_sql = Table::create()
            .table(AlbumDatabase::Table)
            .if_not_exists()
            .col(ColumnDef::new(AlbumDatabase::AlbumId).text().not_null())
            .col(ColumnDef::new(AlbumDatabase::Hash).text().not_null())
            .primary_key(
                Index::create()
                    .col(AlbumDatabase::AlbumId)
                    .col(AlbumDatabase::Hash),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AlbumDatabase::Table, AlbumDatabase::AlbumId)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .from(AlbumDatabase::Table, AlbumDatabase::Hash)
                    .to(Object::Table, Object::Id)
                    .on_delete(sea_query::ForeignKeyAction::Cascade),
            )
            .build(SqliteQueryBuilder);
        conn.execute(&table_sql, [])?;

        // 2. 使用 SeaQuery 建立 Index
        let idx_sql = Index::create()
            .if_not_exists()
            .name("idx_album_databases_hash")
            .table(AlbumDatabase::Table)
            .col(AlbumDatabase::Hash)
            .to_string(SqliteQueryBuilder);
        conn.execute(&idx_sql, [])?;

        // 3. 使用 SeaQuery 建構 Trigger 內部的邏輯
        let new_album_id = Expr::cust("NEW.album_id");
        let old_album_id = Expr::cust("OLD.album_id");

        // 定義一個 Helper function 來產生 UPDATE 語句
        let build_update_sql = |target_album_id: SimpleExpr| {
            // 3.1 子查詢: Item Count
            let sub_count = Query::select()
                .expr(Expr::count(Expr::col(Asterisk)))
                .from(AlbumDatabase::Table)
                .and_where(Expr::col(AlbumDatabase::AlbumId).eq(target_album_id.clone()))
                .to_owned();

            // 3.2 子查詢: Item Size
            let sub_size = Query::select()
                .expr(Func::coalesce({
                    let values: Vec<sea_query::SimpleExpr> = vec![
                        Func::sum(Func::coalesce({
                            let inner: Vec<sea_query::SimpleExpr> = vec![
                                Expr::col((MetaImage::Table, MetaImage::Size)).into(),
                                Expr::col((MetaVideo::Table, MetaVideo::Size)).into(),
                                Expr::val(0).into(),
                            ];
                            inner
                        }))
                        .into(),
                        Expr::val(0).into(),
                    ];
                    values
                }))
                .from(AlbumDatabase::Table)
                .join(
                    JoinType::Join,
                    Object::Table,
                    Expr::col((AlbumDatabase::Table, AlbumDatabase::Hash))
                        .eq(Expr::col((Object::Table, Object::Id))),
                )
                .join(
                    JoinType::LeftJoin,
                    MetaImage::Table,
                    Expr::col((Object::Table, Object::Id))
                        .eq(Expr::col((MetaImage::Table, MetaImage::Id))),
                )
                .join(
                    JoinType::LeftJoin,
                    MetaVideo::Table,
                    Expr::col((Object::Table, Object::Id))
                        .eq(Expr::col((MetaVideo::Table, MetaVideo::Id))),
                )
                .and_where(
                    Expr::col((AlbumDatabase::Table, AlbumDatabase::AlbumId))
                        .eq(target_album_id.clone()),
                )
                .to_owned();

            // 3.3 子查詢: Start Time / End Time
            let sub_time = |func: fn(Expr) -> FunctionCall| {
                Query::select()
                    .expr(func(Expr::col((Object::Table, Object::CreatedTime))))
                    .from(AlbumDatabase::Table)
                    .join(
                        JoinType::Join,
                        Object::Table,
                        Expr::col((AlbumDatabase::Table, AlbumDatabase::Hash))
                            .eq(Expr::col((Object::Table, Object::Id))),
                    )
                    .and_where(
                        Expr::col((AlbumDatabase::Table, AlbumDatabase::AlbumId))
                            .eq(target_album_id.clone()),
                    )
                    .to_owned()
            };

            // 3.4 子查詢: Cover
            let sub_cover_fallback = Query::select()
                .column((Object::Table, Object::Id))
                .from(AlbumDatabase::Table)
                .join(
                    JoinType::Join,
                    Object::Table,
                    Expr::col((AlbumDatabase::Table, AlbumDatabase::Hash))
                        .eq(Expr::col((Object::Table, Object::Id))),
                )
                .and_where(
                    Expr::col((AlbumDatabase::Table, AlbumDatabase::AlbumId))
                        .eq(target_album_id.clone()),
                )
                .order_by((Object::Table, Object::CreatedTime), Order::Asc)
                .limit(1)
                .to_owned();

            let cover_exists_check = Query::select()
                .expr(Expr::val(1))
                .from(AlbumDatabase::Table)
                .and_where(Expr::col(AlbumDatabase::AlbumId).eq(target_album_id.clone()))
                .and_where(Expr::col(AlbumDatabase::Hash).eq(Expr::col(MetaAlbum::Cover)))
                .to_owned();

            // 建構 UPDATE 語句
            Query::update()
                .table(MetaAlbum::Table)
                .value(MetaAlbum::ItemCount, sub_count)
                .value(MetaAlbum::ItemSize, sub_size)
                .value(MetaAlbum::StartTime, sub_time(|expr| Func::min(expr)))
                .value(MetaAlbum::EndTime, sub_time(|expr| Func::max(expr)))
                .value(
                    MetaAlbum::Cover,
                    Expr::case(
                        Expr::col(MetaAlbum::Cover)
                            .is_not_null()
                            .and(Expr::exists(cover_exists_check)),
                        Expr::col(MetaAlbum::Cover),
                    )
                    .finally(sub_cover_fallback),
                )
                .value(
                    MetaAlbum::LastModifiedTime,
                    Expr::cust("strftime('%s', 'now') * 1000"),
                )
                .and_where(Expr::col(MetaAlbum::Id).eq(target_album_id.clone()))
                .to_string(SqliteQueryBuilder)
        };

        // 4. 組合 Raw SQL Trigger
        let update_sql_insert = build_update_sql(new_album_id);
        let update_sql_delete = build_update_sql(old_album_id);

        let table_name = AlbumDatabase::Table.to_string();

        let trigger_sql = format!(
            r#"
            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_insert AFTER INSERT ON "{table_name}"
            BEGIN
                {update_sql_insert};
            END;

            CREATE TRIGGER IF NOT EXISTS update_album_stats_after_delete AFTER DELETE ON "{table_name}"
            BEGIN
                {update_sql_delete};
            END;
        "#
        );

        conn.execute_batch(&trigger_sql)?;
        Ok(())
    }
}
