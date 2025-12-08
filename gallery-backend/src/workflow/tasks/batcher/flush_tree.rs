use log::error;
use mini_executor::BatchTask;
use rusqlite::{Result, Transaction};
use sea_query::{Expr, ExprTrait, OnConflict, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;
use serde_json;

use crate::{
    public::db::tree::TREE,
    public::structure::abstract_data::AbstractData,
    table::{
        meta_album::MetaAlbum,
        meta_image::MetaImage,
        meta_video::MetaVideo,
        object::{Object, ObjectSchema},
        relations::{
            album_database::{AlbumDatabase, AlbumDatabaseSchema},
            database_alias::{DatabaseAlias, DatabaseAliasSchema},
            database_exif::{DatabaseExif, ExifSchema},
            tag_database::{TagDatabase, TagDatabaseSchema},
        },
    },
    workflow::tasks::{BATCH_COORDINATOR, batcher::update_tree::UpdateTreeTask},
};

// [Add] Helper trait 簡化錯誤處理
trait SeaQueryExt<T> {
    fn map_sql_err(self) -> Result<T>;
}

impl<T> SeaQueryExt<T> for std::result::Result<T, sea_query::error::Error> {
    fn map_sql_err(self) -> Result<T> {
        self.map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }
}

#[derive(Debug)]
pub enum FlushOperation {
    InsertAbstractData(AbstractData),
    RemoveAbstractData(AbstractData),
    InsertTag(TagDatabaseSchema),
    RemoveTag(TagDatabaseSchema),
    InsertAlbum(AlbumDatabaseSchema),
    RemoveAlbum(AlbumDatabaseSchema),
    InsertDatabaseAlias(DatabaseAliasSchema),
    InsertExif(ExifSchema),
}

pub struct FlushTreeTask {
    pub operations: Vec<FlushOperation>,
}

impl FlushTreeTask {
    pub fn insert(databases: Vec<AbstractData>) -> Self {
        Self {
            operations: databases
                .into_iter()
                .map(FlushOperation::InsertAbstractData)
                .collect(),
        }
    }
    pub fn remove(databases: Vec<AbstractData>) -> Self {
        Self {
            operations: databases
                .into_iter()
                .map(FlushOperation::RemoveAbstractData)
                .collect(),
        }
    }
}

impl BatchTask for FlushTreeTask {
    async fn batch_run(list: Vec<Self>) {
        // 合併所有任務的操作
        let all_operations: Vec<FlushOperation> =
            list.into_iter().flat_map(|task| task.operations).collect();

        if all_operations.is_empty() {
            return;
        }

        if let Err(e) = flush_tree_task(all_operations) {
            error!("Error in flush_tree_task: {}", e);
        }
    }
}

fn flush_tree_task(operations: Vec<FlushOperation>) -> Result<()> {
    let mut conn = TREE.get_connection().unwrap();
    let tx = conn.transaction()?;

    // --- Helper Logic: 統一處理 Object 表的 Upsert ---
    let upsert_object = |tx: &Transaction, obj: &ObjectSchema, obj_type: &str| -> Result<()> {
        let (sql, values) = Query::insert()
            .into_table(Object::Table)
            .columns([
                Object::Id,
                Object::ObjType,
                Object::CreatedTime,
                Object::Pending,
                Object::Thumbhash,
                Object::Description,
            ])
            .values([
                obj.id.as_str().into(),
                obj_type.into(),
                obj.created_time.into(),
                (obj.pending as i32).into(),
                obj.thumbhash.as_ref().map(|v| v.clone()).into(),
                obj.description.as_ref().map(|s| s.clone()).into(),
            ])
            .map_sql_err()? // 使用 Helper
            .on_conflict({
                let mut oc = OnConflict::column(Object::Id);
                oc.update_columns([
                    Object::ObjType,
                    Object::CreatedTime,
                    Object::Pending,
                    Object::Thumbhash,
                    Object::Description,
                ]);
                oc
            })
            .build_rusqlite(SqliteQueryBuilder);

        tx.execute(sql.as_str(), &*values.as_params())?;
        Ok(())
    };

    // --- Helper Logic: 統一處理相簿關聯同步 ---
    let sync_album_relations =
        |tx: &Transaction,
         hash: &str,
         album_ids: &std::collections::HashSet<arrayvec::ArrayString<64>>|
         -> Result<()> {
            // 1. Delete old relations
            let (sql_del, values_del) = Query::delete()
                .from_table(AlbumDatabase::Table)
                .and_where(Expr::col(AlbumDatabase::Hash).eq(hash)) // 這裡現在因為 ExprTrait 會正確運作
                .build_rusqlite(SqliteQueryBuilder);
            tx.execute(sql_del.as_str(), &*values_del.as_params())?;

            // 2. Insert new relations
            if !album_ids.is_empty() {
                let mut insert_query = Query::insert();
                insert_query
                    .into_table(AlbumDatabase::Table)
                    .columns([AlbumDatabase::AlbumId, AlbumDatabase::Hash]);

                for album_id in album_ids {
                    insert_query
                        .values([album_id.as_str().into(), hash.into()])
                        .map_sql_err()?; // 使用 Helper
                }

                // 這裡使用 Block 讓語法與其他地方一致，避免 clone()
                insert_query.on_conflict({
                    let mut oc = OnConflict::new();
                    oc.do_nothing();
                    oc
                });

                let (sql_ins, values_ins) = insert_query.build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql_ins.as_str(), &*values_ins.as_params())?;
            }
            Ok(())
        };

    for op in operations {
        match op {
            FlushOperation::InsertAbstractData(data) => match data {
                AbstractData::Image(img) => {
                    // 1. Object Upsert
                    upsert_object(&tx, &img.object, "image")?;

                    // 2. Meta Image Upsert
                    let (sql, values) = Query::insert()
                        .into_table(MetaImage::Table)
                        .columns([
                            MetaImage::Id,
                            MetaImage::Size,
                            MetaImage::Width,
                            MetaImage::Height,
                            MetaImage::Ext,
                            MetaImage::Phash,
                        ])
                        .values([
                            img.object.id.as_str().into(),
                            img.metadata.size.into(),
                            img.metadata.width.into(),
                            img.metadata.height.into(),
                            img.metadata.ext.into(),
                            img.metadata.phash.into(),
                        ])
                        .map_sql_err()?
                        .on_conflict({
                            let mut oc = OnConflict::column(MetaImage::Id);
                            oc.update_columns([
                                MetaImage::Size,
                                MetaImage::Width,
                                MetaImage::Height,
                                MetaImage::Ext,
                                MetaImage::Phash,
                            ]);
                            oc
                        })
                        .build_rusqlite(SqliteQueryBuilder);
                    tx.execute(sql.as_str(), &*values.as_params())?;

                    // 3. Sync Album Relations
                    sync_album_relations(&tx, &img.object.id, &img.albums)?;
                }
                AbstractData::Video(vid) => {
                    // 1. Object Upsert
                    upsert_object(&tx, &vid.object, "video")?;

                    // 2. Meta Video Upsert
                    let (sql, values) = Query::insert()
                        .into_table(MetaVideo::Table)
                        .columns([
                            MetaVideo::Id,
                            MetaVideo::Size,
                            MetaVideo::Width,
                            MetaVideo::Height,
                            MetaVideo::Ext,
                            MetaVideo::Duration,
                        ])
                        .values([
                            vid.object.id.as_str().into(),
                            vid.metadata.size.into(),
                            vid.metadata.width.into(),
                            vid.metadata.height.into(),
                            vid.metadata.ext.into(),
                            vid.metadata.duration.into(),
                        ])
                        .map_sql_err()?
                        .on_conflict({
                            let mut oc = OnConflict::column(MetaVideo::Id);
                            oc.update_columns([
                                MetaVideo::Size,
                                MetaVideo::Width,
                                MetaVideo::Height,
                                MetaVideo::Ext,
                                MetaVideo::Duration,
                            ]);
                            oc
                        })
                        .build_rusqlite(SqliteQueryBuilder);
                    tx.execute(sql.as_str(), &*values.as_params())?;

                    // 3. Sync Album Relations
                    sync_album_relations(&tx, &vid.object.id, &vid.albums)?;
                }
                AbstractData::Album(album) => {
                    // 1. Object Upsert
                    upsert_object(&tx, &album.object, "album")?;

                    // 2. Meta Album Upsert
                    let (sql, values) = Query::insert()
                        .into_table(MetaAlbum::Table)
                        .columns([
                            MetaAlbum::Id,
                            MetaAlbum::Title,
                            MetaAlbum::StartTime,
                            MetaAlbum::EndTime,
                            MetaAlbum::LastModifiedTime,
                            MetaAlbum::Cover,
                            MetaAlbum::UserDefinedMetadata,
                            MetaAlbum::ItemCount,
                            MetaAlbum::ItemSize,
                        ])
                        .values([
                            album.object.id.as_str().into(),
                            album.metadata.title.into(),
                            album.metadata.start_time.map(|t| t as i64).into(),
                            album.metadata.end_time.map(|t| t as i64).into(),
                            (album.metadata.last_modified_time as i64).into(),
                            album.metadata.cover.as_ref().map(|c| c.as_str()).into(),
                            serde_json::to_string(&album.metadata.user_defined_metadata)
                                .unwrap_or_default()
                                .into(),
                            (album.metadata.item_count as i64).into(),
                            (album.metadata.item_size as i64).into(),
                        ])
                        .map_sql_err()?
                        .on_conflict({
                            let mut oc = OnConflict::column(MetaAlbum::Id);
                            oc.update_columns([
                                MetaAlbum::Title,
                                MetaAlbum::StartTime,
                                MetaAlbum::EndTime,
                                MetaAlbum::LastModifiedTime,
                                MetaAlbum::Cover,
                                MetaAlbum::UserDefinedMetadata,
                                MetaAlbum::ItemCount,
                                MetaAlbum::ItemSize,
                            ]);
                            oc
                        })
                        .build_rusqlite(SqliteQueryBuilder);
                    tx.execute(sql.as_str(), &*values.as_params())?;
                }
            },

            FlushOperation::RemoveAbstractData(data) => {
                let id = match data {
                    AbstractData::Image(i) => i.object.id,
                    AbstractData::Video(v) => v.object.id,
                    AbstractData::Album(a) => a.object.id,
                };
                let (sql, values) = Query::delete()
                    .from_table(Object::Table)
                    .and_where(Expr::col(Object::Id).eq(id.as_str()))
                    .build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql.as_str(), &*values.as_params())?;
            }

            FlushOperation::InsertTag(schema) => {
                let (sql, values) = Query::insert()
                    .into_table(TagDatabase::Table)
                    .columns([TagDatabase::Hash, TagDatabase::Tag])
                    .values([schema.hash.into(), schema.tag.into()])
                    .map_sql_err()?
                    .on_conflict({
                        let mut oc = OnConflict::new();
                        oc.do_nothing();
                        oc
                    })
                    .build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql.as_str(), &*values.as_params())?;
            }

            FlushOperation::RemoveTag(schema) => {
                let (sql, values) = Query::delete()
                    .from_table(TagDatabase::Table)
                    .and_where(Expr::col(TagDatabase::Hash).eq(schema.hash))
                    .and_where(Expr::col(TagDatabase::Tag).eq(schema.tag))
                    .build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql.as_str(), &*values.as_params())?;
            }

            FlushOperation::InsertAlbum(schema) => {
                let (sql, values) = Query::insert()
                    .into_table(AlbumDatabase::Table)
                    .columns([AlbumDatabase::AlbumId, AlbumDatabase::Hash])
                    .values([schema.album_id.into(), schema.hash.into()])
                    .map_sql_err()?
                    .on_conflict({
                        let mut oc = OnConflict::new();
                        oc.do_nothing();
                        oc
                    })
                    .build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql.as_str(), &*values.as_params())?;
            }

            FlushOperation::RemoveAlbum(schema) => {
                let (sql, values) = Query::delete()
                    .from_table(AlbumDatabase::Table)
                    .and_where(Expr::col(AlbumDatabase::AlbumId).eq(schema.album_id))
                    .and_where(Expr::col(AlbumDatabase::Hash).eq(schema.hash))
                    .build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql.as_str(), &*values.as_params())?;
            }

            FlushOperation::InsertDatabaseAlias(schema) => {
                let (sql, values) = Query::insert()
                    .into_table(DatabaseAlias::Table)
                    .columns([
                        DatabaseAlias::Hash,
                        DatabaseAlias::File,
                        DatabaseAlias::Modified,
                        DatabaseAlias::ScanTime,
                    ])
                    .values([
                        schema.hash.into(),
                        schema.file.into(),
                        schema.modified.into(),
                        schema.scan_time.into(),
                    ])
                    .map_sql_err()?
                    .on_conflict({
                        let mut oc =
                            OnConflict::columns([DatabaseAlias::Hash, DatabaseAlias::ScanTime]);
                        oc.update_columns([
                            DatabaseAlias::File,
                            DatabaseAlias::Modified,
                            DatabaseAlias::ScanTime,
                        ]);
                        oc
                    })
                    .build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql.as_str(), &*values.as_params())?;
            }

            FlushOperation::InsertExif(schema) => {
                let (sql, values) = Query::insert()
                    .into_table(DatabaseExif::Table)
                    .columns([DatabaseExif::Hash, DatabaseExif::Tag, DatabaseExif::Value])
                    .values([schema.hash.into(), schema.tag.into(), schema.value.into()])
                    .map_sql_err()?
                    .on_conflict({
                        let mut oc = OnConflict::columns([DatabaseExif::Hash, DatabaseExif::Tag]);
                        oc.update_column(DatabaseExif::Value);
                        oc
                    })
                    .build_rusqlite(SqliteQueryBuilder);
                tx.execute(sql.as_str(), &*values.as_params())?;
            }
        }
    }

    tx.commit()?;
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
    Ok(())
}
