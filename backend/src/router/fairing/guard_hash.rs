use jsonwebtoken::{DecodingKey, decode};
use log::warn;
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;

use crate::router::claims::claims_hash::ClaimsHash;
use crate::router::claims::claims_timestamp::ClaimsTimestamp;
use crate::router::fairing::VALIDATION;
use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;
use crate::router::{AppResult, GuardError};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::VALIDATION_ALLOW_EXPIRED;
use super::auth_utils::{extract_bearer_token, extract_hash_from_path, my_decode_token};

pub struct GuardHash;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardHash {
    type Error = GuardError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = match extract_bearer_token(req) {
            Ok(token) => token,
            Err(err) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    err.context("Bearer token extraction failed").into(),
                ));
            }
        };

        let claims: ClaimsHash = match my_decode_token(token, &VALIDATION) {
            Ok(claims) => claims,
            Err(err) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    err.context("JWT decoding failed").into(),
                ));
            }
        };

        let data_hash = match extract_hash_from_path(req) {
            Ok(hash) => hash,
            Err(err) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    err.context("Hash extraction failed").into(),
                ));
            }
        };

        // Compare hash in the token with the hash in the request path
        if data_hash != *claims.hash {
            warn!(
                "Hash does not match. Received: {}, Expected: {}.",
                data_hash, claims.hash
            );
            return Outcome::Error((Status::Unauthorized, anyhow!("Hash does not match").into()));
        }
        Outcome::Success(GuardHash)
    }
}

pub struct GuardHashOriginal;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardHashOriginal {
    type Error = GuardError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = match extract_bearer_token(req) {
            Ok(token) => token,
            Err(err) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    err.context("Bearer token extraction failed").into(),
                ));
            }
        };

        let claims: ClaimsHash = match my_decode_token(token, &VALIDATION) {
            Ok(claims) => claims,
            Err(err) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    err.context("JWT decoding failed").into(),
                ));
            }
        };

        if !claims.allow_original {
            warn!("Original hash access is not allowed.");
            return Outcome::Forward(Status::Unauthorized);
        }

        let data_hash = match extract_hash_from_path(req) {
            Ok(hash) => hash,
            Err(err) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    err.context("Hash extraction failed").into(),
                ));
            }
        };

        // Compare hash in the token with the hash in the request path
        if data_hash != *claims.hash {
            warn!(
                "Hash does not match. Received: {}, Expected: {}.",
                data_hash, claims.hash
            );
            return Outcome::Error((Status::Unauthorized, anyhow!("Hash does not match").into()));
        }
        Outcome::Success(GuardHashOriginal)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenewHashToken {
    pub expired_hash_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenewHashTokenReturn {
    pub token: String,
}

#[post("/post/renew-hash-token", format = "json", data = "<token_request>")]
pub async fn renew_hash_token(
    auth: TimestampGuardModified,
    token_request: Json<RenewHashToken>,
) -> AppResult<Json<RenewHashTokenReturn>> {
    tokio::task::spawn_blocking(move || {
        let expired_hash_token = token_request.into_inner().expired_hash_token;
        let token_data = match decode::<ClaimsHash>(
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
                return Err(anyhow::anyhow!("Unauthorized: Invalid token").into());
            }
        };

        if token_data.claims.timestamp != auth.timestamp_decoded {
            warn!(
                "Timestamp does not match. Received: {}, Expected: {}",
                token_data.claims.timestamp, auth.timestamp_decoded
            );
            return Err(anyhow::anyhow!("Unauthorized: Timestamp mismatch").into());
        }

        let claims = token_data.claims;
        let new_hash_claims = ClaimsHash::new(claims.hash, claims.timestamp, claims.allow_original);
        let new_hash_token = new_hash_claims.encode();

        Ok(Json(RenewHashTokenReturn {
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
        let token = match extract_bearer_token(req) {
            Ok(token) => token,
            Err(_) => return Outcome::Forward(Status::Unauthorized),
        };

        let claims: ClaimsTimestamp = match my_decode_token(token, &VALIDATION) {
            Ok(claims) => claims,
            Err(_) => return Outcome::Forward(Status::Unauthorized),
        };

        Outcome::Success(TimestampGuardModified {
            timestamp_decoded: claims.timestamp,
        })
    }
}
