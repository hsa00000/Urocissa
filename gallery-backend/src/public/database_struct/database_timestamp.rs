use serde::{Deserialize, Serialize};

use crate::{public::abstract_data::AbstractData, router::fairing::guard_hash::HashClaims};

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
            AbstractData::Database(database) => {
                HashClaims::new(database.hash, token_timestamp, allow_original).encode()
            }
            AbstractData::Album(album) => {
                if let Some(cover_hash) = album.cover {
                    HashClaims::new(cover_hash, token_timestamp, allow_original).encode()
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
