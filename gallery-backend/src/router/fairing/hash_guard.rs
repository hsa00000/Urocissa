use arrayvec::ArrayString;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, decode, encode};
use log::warn;
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::router::fairing::VALIDATION;
use crate::router::fairing::timestamp_guard::TimestampGuard;
use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;

use super::VALIDATION_ALLOW_EXPIRED;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashClaims {
    pub hash: ArrayString<64>,
    pub timestamp: u128,
    pub exp: u64,
}

impl HashClaims {
    pub fn new(hash: ArrayString<64>, timestamp: u128) -> Self {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 10;

        Self {
            hash,
            timestamp,
            exp,
        }
    }

    pub fn encode(&self) -> String {
        encode(
            &Header::default(),
            &self,
            &EncodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
        )
        .expect("Failed to generate token")
    }
}
pub struct HashGuard;

#[async_trait]
impl<'r> FromRequest<'r> for HashGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Extract token from query parameters
        let token_opt = req
            .uri()
            .query()
            .and_then(|query| query.segments().find(|(key, _)| *key == "token"))
            .and_then(|(_, value)| Some(value));

        let token = match token_opt {
            Some(token) => token,
            None => {
                warn!("Request is missing the 'token' query parameter.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        // Decode the token
        let token_data = match decode::<HashClaims>(
            token,
            &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
            &VALIDATION,
        ) {
            Ok(data) => data,
            Err(err) => {
                warn!("Failed to decode token: {:#?}", err);
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        let claims = token_data.claims;

        // Extract hash from the request URL path
        let hash_opt = req
            .uri()
            .path()
            .segments()
            .last()
            .and_then(|hash_with_ext| hash_with_ext.rsplit_once('.'))
            .map(|(hash, _ext)| hash.to_string());

        let data_hash = match hash_opt {
            Some(hash) => hash,
            None => {
                warn!("No valid 'hash' parameter found in the uri.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        // Compare hash in the token with the hash in the request path
        if data_hash != *claims.hash {
            warn!(
                "Hash does not match. Received: {}, Expected: {}.",
                data_hash, claims.hash
            );
            return Outcome::Forward(Status::Unauthorized);
        }
        Outcome::Success(HashGuard)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRequest {
    pub expired_hash_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenReturn {
    pub new_hash_token: String,
}

#[post(
    "/post/renew-timestamp-token?<timestamp>",
    format = "json",
    data = "<token_request>"
)]
pub async fn renew_hash_token(
    _auth: TimestampGuard,
    timestamp: u128,
    token_request: Json<TokenRequest>,
) -> Result<Json<TokenReturn>, Status> {
    tokio::task::spawn_blocking(move || {
        let expired_hash_token = token_request.into_inner().expired_hash_token;
        let token_data = match decode::<HashClaims>(
            &expired_hash_token,
            &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
            &VALIDATION_ALLOW_EXPIRED,
        ) {
            Ok(data) => data,
            Err(err) => {
                warn!(
                    "Token renewal failed: unable to decode token. Error: {:#?}",
                    err
                );
                return Err(Status::Unauthorized);
            }
        };

        if token_data.claims.timestamp != timestamp {
            warn!(
                "Timestamp does not match. Received: {}, Expected: {}.",
                token_data.claims.timestamp, timestamp
            );
            return Err(Status::Unauthorized);
        }

        let claims = token_data.claims;
        let new_hash_claims = HashClaims::new(claims.hash, claims.timestamp);
        let new_hash_token = new_hash_claims.encode();

        Ok(Json(TokenReturn { new_hash_token }))
    })
    .await
    .unwrap()
}
