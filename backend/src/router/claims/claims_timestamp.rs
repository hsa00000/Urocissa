use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::public::structure::album::ResolvedShare;
use crate::router::post::authenticate::get_jwt_secret_key;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimsTimestamp {
    pub resolved_share_opt: Option<ResolvedShare>,
    pub timestamp: u128,
    pub exp: u64,
}

impl ClaimsTimestamp {
    pub fn new(resolved_share_opt: Option<ResolvedShare>, timestamp: u128) -> Self {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 300;

        Self {
            resolved_share_opt,
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
