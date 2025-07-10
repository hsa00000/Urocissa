use jsonwebtoken::{DecodingKey, decode};
use log::warn;
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

use crate::router::AppResult;
use crate::router::claims::claims_timestamp::ClaimsTimestamp;
use crate::router::fairing::VALIDATION;
use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;

use super::VALIDATION_ALLOW_EXPIRED;
use super::guard_share::GuardShare;
pub struct GuardTimestamp {
    pub claims: ClaimsTimestamp,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardTimestamp {
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

        let token_data = match decode::<ClaimsTimestamp>(
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
        let query_timestamp = req.uri().query().and_then(|query| {
            query
                .segments()
                .find(|(key, _)| *key == "timestamp")
                .and_then(|(_, value)| value.parse::<u128>().ok())
        });

        let query_timestamp = match query_timestamp {
            Some(ts) => ts,
            None => {
                warn!("No valid 'timestamp' parameter found in the query.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        if query_timestamp != claims.timestamp {
            warn!(
                "Timestamp does not match. Received: {}, Expected: {}.",
                query_timestamp, claims.timestamp
            );
            return Outcome::Forward(Status::Unauthorized);
        }

        Outcome::Success(GuardTimestamp { claims })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenewTimestampToken {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenewTimestampTokenReturn {
    pub token: String,
}

#[post(
    "/post/renew-timestamp-token",
    format = "json",
    data = "<token_request>"
)]
pub async fn renew_timestamp_token(
    _auth: GuardShare,
    token_request: Json<RenewTimestampToken>,
) -> AppResult<Json<RenewTimestampTokenReturn>> {
    tokio::task::spawn_blocking(move || {
        let token = token_request.into_inner().token;
        let token_data = match decode::<ClaimsTimestamp>(
            &token,
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

        let claims = token_data.claims;
        let new_claims = ClaimsTimestamp::new(claims.resolved_share_opt, claims.timestamp);
        let new_token = new_claims.encode();

        Ok(Json(RenewTimestampTokenReturn { token: new_token }))
    })
    .await
    .unwrap()
}