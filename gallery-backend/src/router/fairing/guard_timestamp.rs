use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use log::{error, info, warn};
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;

use super::guard_auth::AuthGuard;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampClaims {
    pub timestamp: u128,
    pub exp: u64,
}

impl TimestampClaims {
    pub fn new(timestamp: u128) -> Self {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 10;

        Self { timestamp, exp }
    }
    pub fn encode(&self) -> String {
        // Encode the JWT token
        let token = encode(
            &Header::default(),
            &self,
            &EncodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
        )
        .expect("Token generation failed");

        return token;
    }
}

pub struct TimestampGuard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TimestampGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = match req.headers().get_one("Authorization") {
            Some(header) => header,
            None => {
                warn!("Authorization header not found.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        let token = match auth_header.strip_prefix("Bearer ") {
            Some(token) => token,
            None => {
                warn!("Authorization header is malformed. Expected format: 'Bearer <token>'");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        let token_data = match decode::<TimestampClaims>(
            token,
            &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(data) => data,
            Err(err) => {
                warn!("TimestampCLaims decode failed: {:?}", err);
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
                warn!("Timestamp query parameter not found or invalid.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        if query_timestamp != claims.timestamp {
            warn!(
                "Timestamp mismatch: query = {}, claims = {}",
                query_timestamp, claims.timestamp
            );
            return Outcome::Forward(Status::Unauthorized);
        }

        let current_time = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(_) => {
                error!("System time error while checking token expiration.");
                return Outcome::Forward(Status::Unauthorized);
            }
        };

        if claims.exp < current_time {
            error!("Token expired.");
            return Outcome::Forward(Status::Unauthorized);
        }

        info!("Timestamp token passed.");
        Outcome::Success(TimestampGuard)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRequest {
    pub token: String,
}

#[post("/post/renew-timestamp-token", format = "json", data = "<token>")]
pub async fn renew_timestamp_token(
    _auth: AuthGuard,
    token: Json<TokenRequest>,
) -> Result<Json<String>, Status> {
    tokio::task::spawn_blocking(move || {
        let token_data = match decode::<TimestampClaims>(
            &token.into_inner().token,
            &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(data) => data,
            Err(err) => {
                warn!("Renew timestamp failed: {:?}", err);
                return Err(Status::Unauthorized);
            }
        };

        let claims = token_data.claims;
        let new_claims = TimestampClaims::new(claims.timestamp);
        let new_token = new_claims.encode();

        info!("new_token {}", new_token);

        Ok(Json(new_token))
    })
    .await
    .unwrap()
}
