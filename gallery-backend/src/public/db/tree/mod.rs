pub mod new;
pub mod read_tags;

use anyhow::Result;
use redb::{Database, ReadableDatabase};

use crate::public::structure::abstract_data::AbstractData;
use crate::table::album::AlbumCombined;
use crate::table::image::ImageCombined;
use crate::table::object::{ObjectSchema, ObjectType, OBJECT_TABLE};
use crate::table::video::VideoCombined;
use std::sync::{atomic::AtomicU64, Arc, LazyLock, RwLock};

pub struct Tree {
    pub in_disk: Database,
    pub in_memory: &'static Arc<RwLock<Vec<AbstractData>>>,
}

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree::new());

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl Tree {
    pub fn begin_read(&self) -> Result<redb::ReadTransaction> {
        Ok(self.in_disk.begin_read()?)
    }

    pub fn begin_write(&self) -> Result<redb::WriteTransaction> {
        Ok(self.in_disk.begin_write()?)
    }
    pub fn load_from_db(&self, id: impl AsRef<str>) -> Result<AbstractData> {
        let id = id.as_ref();
        let txn = self.in_disk.begin_read()?;

        // 先查詢 Object 表確認類型
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let obj_bytes = obj_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("No data found for id: {}", id))?;
        
        let obj_schema: ObjectSchema = bitcode::decode(obj_bytes.value())?;

        match obj_schema.obj_type {
            ObjectType::Album => {
                let album = AlbumCombined::get_by_id(&txn, id)?;
                Ok(AbstractData::Album(album))
            }
            ObjectType::Image => {
                let image = ImageCombined::get_by_id(&txn, id)?;
                Ok(AbstractData::Image(image))
            }
            ObjectType::Video => {
                let video = VideoCombined::get_by_id(&txn, id)?;
                Ok(AbstractData::Video(video))
            }
        }
    }

    pub fn load_all_data_from_db(&self) -> Result<Vec<AbstractData>> {
        let txn = self.in_disk.begin_read()?;

        let all_images = ImageCombined::get_all(&txn)?;
        let all_videos = VideoCombined::get_all(&txn)?;

        let mut result = Vec::with_capacity(all_images.len() + all_videos.len());

        for image in all_images {
            result.push(AbstractData::Image(image));
        }

        for video in all_videos {
            result.push(AbstractData::Video(video));
        }

        Ok(result)
    }

    pub fn load_data_from_hash(&self, hash: impl AsRef<str>) -> Result<Option<AbstractData>> {
        let hash = hash.as_ref();
        match self.load_from_db(hash) {
            Ok(data) => Ok(Some(data)),
            Err(_) => Ok(None),
        }
    }
}
