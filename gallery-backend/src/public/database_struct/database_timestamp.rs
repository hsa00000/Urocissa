use serde::{Deserialize, Serialize};

use crate::{public::abstract_data::AbstractData, router::fairing::guard_hash::HashClaims};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataBaseTimestamp {
    pub abstract_data: AbstractData,
    pub timestamp: u128,
    pub token: String,
}

impl DataBaseTimestamp {
    pub fn new(abstract_data: AbstractData, priority_list: &[&str]) -> Self {
        let timestamp = abstract_data.compute_timestamp(priority_list);
        let token = match &abstract_data {
            AbstractData::Database(database) => HashClaims::new(database.hash).encode(),
            AbstractData::Album(album) => {
                if let Some(cover_hash) = album.cover {
                    HashClaims::new(cover_hash).encode()
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
