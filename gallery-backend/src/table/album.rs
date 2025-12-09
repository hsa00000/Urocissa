use crate::table::meta_album::{AlbumMetadataSchema, META_ALBUM_TABLE};
use crate::table::object::{ObjectSchema, ObjectType, OBJECT_TABLE};
use crate::table::relations::tag_database::TagDatabase;
use anyhow::Result;
use redb::ReadTransaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: AlbumMetadataSchema,
}

impl AlbumCombined {
    pub fn get_by_id(txn: &ReadTransaction, id: impl AsRef<str>) -> Result<Self> {
        let id = id.as_ref();
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let meta_table = txn.open_table(META_ALBUM_TABLE)?;

        let obj_bytes = obj_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Object not found"))?;
        let mut object: ObjectSchema = bitcode::decode(obj_bytes.value())?;

        let meta_bytes = meta_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Metadata not found"))?;
        let metadata: AlbumMetadataSchema = bitcode::decode(meta_bytes.value())?;

        // 讀取 Tags
        object.tags = TagDatabase::fetch_tags(txn, id)?;

        Ok(Self { object, metadata })
    }

    pub fn get_all(txn: &ReadTransaction) -> Result<Vec<Self>> {
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let meta_table = txn.open_table(META_ALBUM_TABLE)?;
        let mut albums = Vec::new();

        // 掃描 MetaAlbum 即可，因為只有 Album 會有 MetaAlbum
        for entry in meta_table.range::<&str>(..)? {
            let (id, meta_val) = entry?;
            let id_str = id.value();

            if let Some(obj_val) = obj_table.get(id_str)? {
                let object: ObjectSchema = bitcode::decode(obj_val.value())?;
                let metadata: AlbumMetadataSchema = bitcode::decode(meta_val.value())?;

                if object.obj_type == ObjectType::Album {
                    albums.push(Self { object, metadata });
                }
            }
        }

        // 批次讀取 Tags
        let mut tag_map = TagDatabase::fetch_all_tags(txn)?;
        for album in &mut albums {
            if let Some(tags) = tag_map.remove(&album.object.id) {
                album.object.tags = tags;
            }
        }

        Ok(albums)
    }
}
