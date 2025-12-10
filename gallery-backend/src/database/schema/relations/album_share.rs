use crate::database::schema::meta_album::{AlbumMetadataSchema, META_ALBUM_TABLE};
use anyhow::Result;
use arrayvec::ArrayString;
use redb::{ReadTransaction, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Key: (AlbumId, Url) -> Value: Share (Serialized)
pub const ALBUM_SHARE_TABLE: TableDefinition<(&str, &str), &[u8]> =
    TableDefinition::new("album_share");

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash, bitcode::Encode, bitcode::Decode)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub url: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: u64,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash, bitcode::Encode, bitcode::Decode)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedShare {
    #[serde(flatten)]
    pub share: Share,
    pub album_id: ArrayString<64>,
    pub album_title: Option<String>,
}

pub struct AlbumShareTable;

impl AlbumShareTable {
    pub fn get_all_shares_grouped(
        txn: &ReadTransaction,
    ) -> Result<HashMap<String, HashMap<ArrayString<64>, Share>>> {
        let table = txn.open_table(ALBUM_SHARE_TABLE)?;
        let mut map: HashMap<String, HashMap<ArrayString<64>, Share>> = HashMap::new();

        for entry in table.range::<(&str, &str)>(..)? {
            let (key_guard, value) = entry?;
            let key = key_guard.value();
            let (album_id, _) = key;
            let share: Share = bitcode::decode(value.value())?;
            map.entry(album_id.to_string())
                .or_default()
                .insert(share.url, share);
        }
        Ok(map)
    }

    pub fn get_all_resolved(txn: &ReadTransaction) -> Result<Vec<ResolvedShare>> {
        let share_table = txn.open_table(ALBUM_SHARE_TABLE)?;
        let album_table = txn.open_table(META_ALBUM_TABLE)?;
        let mut result = Vec::new();

        for entry in share_table.range::<(&str, &str)>(..)? {
            let (key_guard, value) = entry?;
            let key = key_guard.value();
            let (album_id, _) = key;
            let share: Share = bitcode::decode(value.value())?;

            // 讀取相簿標題
            let mut album_title = None;
            if let Some(album_bytes) = album_table.get(album_id)? {
                let album: AlbumMetadataSchema = bitcode::decode(album_bytes.value())?;
                album_title = album.title;
            }

            if let Ok(aid) = ArrayString::from(album_id) {
                result.push(ResolvedShare {
                    share,
                    album_id: aid,
                    album_title,
                });
            }
        }
        Ok(result)
    }
}
