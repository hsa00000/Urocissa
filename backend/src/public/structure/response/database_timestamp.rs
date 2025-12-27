use serde::{Deserialize, Serialize};

use crate::{
    public::structure::abstract_data::AbstractData, router::claims::claims_hash::ClaimsHash,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseTimestamp {
    pub abstract_data: AbstractData,
    pub timestamp: u128,
}

impl DatabaseTimestamp {
    pub fn new(abstract_data: AbstractData, priority_list: &[&str]) -> Self {
        let timestamp = abstract_data.compute_timestamp(priority_list);
        Self {
            abstract_data,
            timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataBaseTimestampReturn {
    pub abstract_data: AbstractData,
    pub timestamp: u128,
    pub token: String,
}

impl DataBaseTimestampReturn {
    pub fn new(
        abstract_data: AbstractData,
        priority_list: &[&str],
        token_timestamp: u128,
        allow_original: bool,
    ) -> Self {
        let timestamp = abstract_data.compute_timestamp(priority_list);
        let token = match &abstract_data {
            AbstractData::Image(img) => {
                ClaimsHash::new(img.object.id, token_timestamp, allow_original).encode()
            }
            AbstractData::Video(vid) => {
                ClaimsHash::new(vid.object.id, token_timestamp, allow_original).encode()
            }
            AbstractData::Album(alb) => {
                if let Some(cover_hash) = alb.metadata.cover {
                    ClaimsHash::new(cover_hash, token_timestamp, allow_original).encode()
                } else {
                    String::new()
                }
            }
        };
        Self {
            abstract_data,
            timestamp,
            token,
        }
    }
}
