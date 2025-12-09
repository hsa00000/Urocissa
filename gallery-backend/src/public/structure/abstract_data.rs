use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};

use crate::public::db::tree::TREE;
use crate::table::album::AlbumCombined;
use crate::table::image::ImageCombined;
use crate::table::video::VideoCombined;

use super::database::file_modify::FileModify;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbstractDataResponse {
    pub data: AbstractData,
    pub alias: Vec<FileModify>,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)] // 這很重要，讓 JSON 輸出時不會多一層 Key，直接根據內容判斷
pub enum AbstractData {
    Image(ImageCombined),
    Video(VideoCombined),
    Album(AlbumCombined),
}

impl AbstractData {
    pub fn compute_timestamp(self: &Self) -> i64 {
        match self {
            AbstractData::Image(image) => image.object.created_time,
            AbstractData::Video(video) => video.object.created_time,
            AbstractData::Album(album) => album.object.created_time,
        }
    }
    pub fn hash(self: &Self) -> ArrayString<64> {
        match self {
            AbstractData::Image(image) => image.object.id,
            AbstractData::Video(video) => video.object.id,
            AbstractData::Album(album) => album.object.id,
        }
    }
    pub fn width(self: &Self) -> u32 {
        match self {
            AbstractData::Image(image) => image.metadata.width,
            AbstractData::Video(video) => video.metadata.width,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn height(self: &Self) -> u32 {
        match self {
            AbstractData::Image(image) => image.metadata.height,
            AbstractData::Video(video) => video.metadata.height,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn alias(self: &Self) -> Vec<FileModify> {
        match self {
            AbstractData::Image(image) => Self::fetch_alias(&image.object.id),
            AbstractData::Video(video) => Self::fetch_alias(&video.object.id),
            AbstractData::Album(_) => vec![],
        }
    }

    fn fetch_alias(hash: &ArrayString<64>) -> Vec<FileModify> {
        let txn = TREE.begin_read().unwrap();
        let table = txn.open_table(crate::table::relations::database_alias::DATABASE_ALIAS_TABLE).unwrap();
        let mut aliases = Vec::new();
        
        // Iterate through all entries with the given hash prefix
        let range_start = (hash.as_str(), i64::MIN);
        let range_end = (hash.as_str(), i64::MAX);
        let mut iter = table.range(range_start..=range_end).unwrap().rev(); // rev() for DESC order
        
        while let Some(Ok((_, value))) = iter.next() {
            let alias: crate::table::relations::database_alias::DatabaseAliasSchema = bitcode::decode(value.value()).unwrap();
            aliases.push(FileModify {
                file: alias.file,
                modified: alias.modified as u128,
                scan_time: alias.scan_time as u128,
            });
        }
        
        aliases
    }

    pub fn generate_token(&self, timestamp: u128, allow_original: bool) -> String {
        match self {
            AbstractData::Image(image) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(image.object.id, timestamp, allow_original).encode()
            }
            AbstractData::Video(video) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(video.object.id, timestamp, allow_original).encode()
            }
            AbstractData::Album(album) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                // If the album has a cover, we must sign the cover hash
                // because the frontend will use this token to request the cover image file.
                // If there is no cover, we fallback to ID (though no image will be fetched).
                let hash = album.metadata.cover.unwrap_or(album.object.id);
                ClaimsHash::new(hash, timestamp, allow_original).encode()
            }
        }
    }

    pub fn to_response(self, timestamp: u128, allow_original: bool) -> AbstractDataResponse {
        let alias = self.alias();
        let token = self.generate_token(timestamp, allow_original);
        AbstractDataResponse {
            data: self,
            alias,
            token,
        }
    }
}
