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
    pub fn new(abstract_data: AbstractData) -> Self {
        let timestamp = abstract_data.compute_timestamp();
        Self {
            abstract_data,
            timestamp: timestamp as u128,
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
    pub fn new(abstract_data: AbstractData, token_timestamp: u128, allow_original: bool) -> Self {
        let timestamp = abstract_data.compute_timestamp();
        let token = match &abstract_data {
            AbstractData::Database(database) => {
                ClaimsHash::new(database.hash, token_timestamp, allow_original).encode()
            }
            AbstractData::Album(album) => {
                if let Some(cover_hash) = album.cover {
                    ClaimsHash::new(cover_hash, token_timestamp, allow_original).encode()
                } else {
                    String::new()
                }
            }
        };
        Self {
            abstract_data,
            timestamp: timestamp as u128,
            token,
        }
    }
}
