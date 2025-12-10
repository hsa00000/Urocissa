use crate::database::schema::meta_video::{VideoMetadataSchema, META_VIDEO_TABLE};
use crate::database::schema::object::{ObjectSchema, ObjectType, OBJECT_TABLE};
use crate::database::schema::relations::album_database::AlbumDatabase;
use crate::database::schema::relations::database_exif::DatabaseExif;
use crate::database::schema::relations::tag_database::TagDatabase;
use anyhow::Result;
use arrayvec::ArrayString;
use redb::ReadTransaction;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: VideoMetadataSchema,
    #[serde(default)]
    pub albums: HashSet<ArrayString<64>>,
    #[serde(default, rename = "exifVec")]
    pub exif_vec: BTreeMap<String, String>,
}

impl VideoCombined {
    pub fn get_by_id(txn: &ReadTransaction, id: impl AsRef<str>) -> Result<Self> {
        let id = id.as_ref();
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let meta_table = txn.open_table(META_VIDEO_TABLE)?;

        let obj_bytes = obj_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Object not found"))?;
        let mut object: ObjectSchema = bitcode::decode(obj_bytes.value())?;

        let meta_bytes = meta_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Video Metadata not found"))?;
        let metadata: VideoMetadataSchema = bitcode::decode(meta_bytes.value())?;

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
        let meta_table = txn.open_table(META_VIDEO_TABLE)?;
        let mut videos = Vec::new();

        for entry in meta_table.range::<&str>(..)? {
            let (id, meta_val) = entry?;
            let id_str = id.value();

            if let Some(obj_val) = obj_table.get(id_str)? {
                let object: ObjectSchema = bitcode::decode(obj_val.value())?;
                let metadata: VideoMetadataSchema = bitcode::decode(meta_val.value())?;

                if object.obj_type == ObjectType::Video {
                    videos.push(Self {
                        object,
                        metadata,
                        albums: HashSet::new(),
                        exif_vec: BTreeMap::new(),
                    });
                }
            }
        }

        if videos.is_empty() {
            return Ok(videos);
        }

        let mut album_map = AlbumDatabase::fetch_all_albums(txn)?;
        let mut tag_map = TagDatabase::fetch_all_tags(txn)?;
        let mut exif_map = DatabaseExif::fetch_all_exif(txn)?;

        for video in &mut videos {
            if let Some(albums) = album_map.remove(&video.object.id) {
                video.albums = albums;
            }
            if let Some(tags) = tag_map.remove(&video.object.id) {
                video.object.tags = tags;
            }
            if let Some(exif) = exif_map.remove(&video.object.id) {
                video.exif_vec = exif;
            }
        }

        Ok(videos)
    }
}
