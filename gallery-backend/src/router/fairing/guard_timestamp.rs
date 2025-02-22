use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use serde::{Deserialize, Serialize};

use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampClaims {
    pub timestamp: u128,
    pub exp: u64,
}

pub struct TimestampGuard {
    claims: TimestampClaims,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TimestampGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(auth_header) = req.headers().get_one("Authorization") {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                let validation = Validation::new(Algorithm::HS256);

                match decode::<TimestampClaims>(
                    token,
                    &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
                    &validation,
                ) {
                    Ok(token_data_claims) => {
                        let claims = token_data_claims.claims;

                        info!("timestamp token passed");
                        return Outcome::Success(TimestampGuard { claims });
                    }
                    Err(err) => {
                        warn!("JWT validation failed: {:?}", err);
                    }
                }
            } else {
                warn!("Authorization header is malformed. Expected format: 'Bearer <token>'");
            }
        } else {
            warn!("Authorization header not found.");
        }

        Outcome::Forward(Status::Unauthorized)
    }
}
