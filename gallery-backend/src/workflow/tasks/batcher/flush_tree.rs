use log::error;
use mini_executor::BatchTask;
use anyhow::Result;

use crate::{
    public::db::tree::TREE,
    public::structure::abstract_data::AbstractData,
    table::{
        object::OBJECT_TABLE,
        meta_image::META_IMAGE_TABLE,
        meta_video::META_VIDEO_TABLE,
        meta_album::META_ALBUM_TABLE,
        relations::{
            album_database::{AlbumDatabase, AlbumItemSchema},
            database_alias::{DatabaseAliasSchema, DATABASE_ALIAS_TABLE},
            database_exif::{ExifSchema, DATABASE_EXIF_TABLE},
            tag_database::{TagDatabase, TagDatabaseSchema},
        },
    },
    // [Fix] 補回引用
    workflow::tasks::{BATCH_COORDINATOR, batcher::update_tree::UpdateTreeTask},
};

#[derive(Debug)]
pub enum FlushOperation {
    InsertAbstractData(AbstractData),
    RemoveAbstractData(AbstractData),
    InsertTag(TagDatabaseSchema),
    RemoveTag(TagDatabaseSchema),
    InsertAlbum(AlbumItemSchema),
    RemoveAlbum(AlbumItemSchema),
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
    let mut tx = TREE.begin_write()?;

    for op in operations {
        match op {
            FlushOperation::InsertAbstractData(data) => {
                let id = data.hash();
                let id_str = id.as_str();

                // 1. 寫入 Object Table
                match &data {
                    AbstractData::Image(i) => {
                        let obj_bytes = bitcode::encode(&i.object);
                        tx.open_table(OBJECT_TABLE)?.insert(id_str, obj_bytes.as_slice())?;
                        
                        let meta_bytes = bitcode::encode(&i.metadata);
                        tx.open_table(META_IMAGE_TABLE)?.insert(id_str, meta_bytes.as_slice())?;
                        
                        for tag in &i.object.tags {
                            TagDatabase::add_tag(&mut tx, id_str, tag)?;
                        }
                        for album_id in &i.albums {
                            AlbumDatabase::add_item(&mut tx, album_id, id_str)?;
                        }
                    }
                    AbstractData::Video(v) => {
                        let obj_bytes = bitcode::encode(&v.object);
                        tx.open_table(OBJECT_TABLE)?.insert(id_str, obj_bytes.as_slice())?;
                        
                        let meta_bytes = bitcode::encode(&v.metadata);
                        tx.open_table(META_VIDEO_TABLE)?.insert(id_str, meta_bytes.as_slice())?;

                        for tag in &v.object.tags {
                            TagDatabase::add_tag(&mut tx, id_str, tag)?;
                        }
                        for album_id in &v.albums {
                            AlbumDatabase::add_item(&mut tx, album_id, id_str)?;
                        }
                    }
                    AbstractData::Album(a) => {
                        let obj_bytes = bitcode::encode(&a.object);
                        tx.open_table(OBJECT_TABLE)?.insert(id_str, obj_bytes.as_slice())?;
                        
                        let meta_bytes = bitcode::encode(&a.metadata);
                        tx.open_table(META_ALBUM_TABLE)?.insert(id_str, meta_bytes.as_slice())?;

                        for tag in &a.object.tags {
                            TagDatabase::add_tag(&mut tx, id_str, tag)?;
                        }
                    }
                }
            }
            FlushOperation::RemoveAbstractData(data) => {
                let id = data.hash();
                let id_str = id.as_str();

                tx.open_table(OBJECT_TABLE)?.remove(id_str)?;
                match data {
                    AbstractData::Image(_) => { tx.open_table(META_IMAGE_TABLE)?.remove(id_str)?; },
                    AbstractData::Video(_) => { tx.open_table(META_VIDEO_TABLE)?.remove(id_str)?; },
                    AbstractData::Album(_) => { tx.open_table(META_ALBUM_TABLE)?.remove(id_str)?; },
                }
            }
            FlushOperation::InsertTag(schema) => {
                TagDatabase::add_tag(&mut tx, &schema.hash, &schema.tag)?;
            }
            FlushOperation::RemoveTag(schema) => {
                TagDatabase::remove_tag(&mut tx, &schema.hash, &schema.tag)?;
            }
            FlushOperation::InsertAlbum(schema) => {
                AlbumDatabase::add_item(&mut tx, &schema.album_id, &schema.hash)?;
            }
            FlushOperation::RemoveAlbum(schema) => {
                AlbumDatabase::remove_item(&mut tx, &schema.album_id, &schema.hash)?;
            }
            FlushOperation::InsertDatabaseAlias(schema) => {
                let bytes = bitcode::encode(&schema);
                tx.open_table(DATABASE_ALIAS_TABLE)?.insert(
                    (schema.hash.as_str(), schema.scan_time), 
                    bytes.as_slice()
                )?;
            }
            FlushOperation::InsertExif(schema) => {
                tx.open_table(DATABASE_EXIF_TABLE)?.insert(
                    (schema.hash.as_str(), schema.tag.as_str()),
                    schema.value.as_str()
                )?;
            }
        }
    }

    tx.commit()?;

    // [Fix] 補回：觸發記憶體緩存更新
    // 這是讓 TUI 和 API 能夠讀取到新寫入資料的關鍵
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);

    Ok(())
}
