use arrayvec::ArrayString;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::router::post::authenticate::get_jwt_secret_key;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimsHash {
    pub allow_original: bool,
    pub hash: ArrayString<64>,
    pub timestamp: u128,
    pub exp: u64,
}

impl ClaimsHash {
    pub fn new(hash: ArrayString<64>, timestamp: u128, allow_original: bool) -> Self {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 300;

        Self {
            allow_original,
            hash,
            timestamp,
            exp,
        }
    }

    pub fn encode(&self) -> String {
        encode(
            &Header::default(),
            &self,
            &EncodingKey::from_secret(&get_jwt_secret_key()),
        )
        .expect("Failed to generate token")
    }
}
