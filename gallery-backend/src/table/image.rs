use crate::table::meta_image::{ImageMetadataSchema, META_IMAGE_TABLE};
use crate::table::object::{ObjectSchema, ObjectType, OBJECT_TABLE};
use crate::table::relations::album_database::AlbumDatabase;
use crate::table::relations::database_exif::DatabaseExif;
use crate::table::relations::tag_database::TagDatabase;
use anyhow::Result;
use arrayvec::ArrayString;
use redb::ReadTransaction;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: ImageMetadataSchema,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
}

impl ImageCombined {
    pub fn get_by_id(txn: &ReadTransaction, id: impl AsRef<str>) -> Result<Self> {
        let id = id.as_ref();
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let meta_table = txn.open_table(META_IMAGE_TABLE)?;

        let obj_bytes = obj_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Object not found"))?;
        let mut object: ObjectSchema = bitcode::decode(obj_bytes.value())?;

        let meta_bytes = meta_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Image Metadata not found"))?;
        let metadata: ImageMetadataSchema = bitcode::decode(meta_bytes.value())?;

        let albums = AlbumDatabase::fetch_albums(txn, id)?;
        object.tags = TagDatabase::fetch_tags(txn, id)?;
        let exif_vec = DatabaseExif::fetch_exif(txn, id)?;

        Ok(Self {
            object,
            metadata,
            albums,
            exif_vec,
        })
    }

    pub fn get_all(txn: &ReadTransaction) -> Result<Vec<Self>> {
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let meta_table = txn.open_table(META_IMAGE_TABLE)?;
        let mut images = Vec::new();

        for entry in meta_table.range::<&str>(..)? {
            let (id, meta_val) = entry?;
            let id_str = id.value();

            if let Some(obj_val) = obj_table.get(id_str)? {
                let object: ObjectSchema = bitcode::decode(obj_val.value())?;
                let metadata: ImageMetadataSchema = bitcode::decode(meta_val.value())?;

                if object.obj_type == ObjectType::Image {
                    images.push(Self {
                        object,
                        metadata,
                        albums: HashSet::new(),
                        exif_vec: BTreeMap::new(),
                    });
                }
            }
        }

        if images.is_empty() {
            return Ok(images);
        }

        let mut album_map = AlbumDatabase::fetch_all_albums(txn)?;
        let mut tag_map = TagDatabase::fetch_all_tags(txn)?;
        let mut exif_map = DatabaseExif::fetch_all_exif(txn)?;

        for image in &mut images {
            if let Some(albums) = album_map.remove(&image.object.id) {
                image.albums = albums;
            }
            if let Some(tags) = tag_map.remove(&image.object.id) {
                image.object.tags = tags;
            }
            if let Some(exif) = exif_map.remove(&image.object.id) {
                image.exif_vec = exif;
            }
        }

        Ok(images)
    }
}
