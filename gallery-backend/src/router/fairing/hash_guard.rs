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
use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;

use super::VALIDATION_ALLOW_EXPIRED;
use super::timestamp_guard::TimestampClaims;

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
            + 1;

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
        let auth_header = match req.headers().get_one("Authorization") {
            Some(header) => header,
            None => {
                warn!("Request is missing the Authorization header.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        let token = match auth_header.strip_prefix("Bearer ") {
            Some(token) => token,
            None => {
                warn!("Authorization header format is invalid. Expected 'Bearer <token>'.");
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
    pub token: String,
}

#[post("/post/renew-hash-token", format = "json", data = "<token_request>")]
pub async fn renew_hash_token(
    auth: TimestampGuardModified,
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

        if token_data.claims.timestamp != auth.timestamp_decoded {
            warn!(
                "Timestamp does not match. Received: {}, Expected: {}.",
                token_data.claims.timestamp, auth.timestamp_decoded
            );
            return Err(Status::Unauthorized);
        }

        let claims = token_data.claims;
        let new_hash_claims = HashClaims::new(claims.hash, claims.timestamp);
        let new_hash_token = new_hash_claims.encode();

        Ok(Json(TokenReturn {
            token: new_hash_token,
        }))
    })
    .await
    .unwrap()
}

pub struct TimestampGuardModified {
    pub timestamp_decoded: u128,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TimestampGuardModified {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = match req.headers().get_one("Authorization") {
            Some(header) => header,
            None => {
                warn!("Request is missing the Authorization header.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        let token = match auth_header.strip_prefix("Bearer ") {
            Some(token) => token,
            None => {
                warn!("Authorization header format is invalid. Expected 'Bearer <token>'.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        let token_data = match decode::<TimestampClaims>(
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

        Outcome::Success(TimestampGuardModified {
            timestamp_decoded: claims.timestamp,
        })
    }
}
