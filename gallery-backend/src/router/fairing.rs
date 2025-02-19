use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use rocket::fairing::AdHoc;
use rocket::http::{CookieJar, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

use crate::public::config::PUBLIC_CONFIG;

use super::post::authenticate::{Claims, JSON_WEB_TOKEN_SECRET_KEY};

pub fn cache_control_fairing() -> AdHoc {
    AdHoc::on_response("Add Cache-Control header", |req, res| {
        Box::pin(async move {
            // Check if the response status is successful (2xx status codes)
            if res.status().code >= 200 && res.status().code < 300 {
                // Apply cache control headers based on the request path
                if req.uri().path().starts_with("/object")
                    || req.uri().path().starts_with("/assets")
                    || req.uri().path().starts_with("/favicon.ico")
                {
                    res.set_raw_header("Cache-Control", "max-age=31536000, public");
                }
            }
        })
    })
}

pub struct AuthGuard {
    pub claims: Claims,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Check for JWT cookie
        let cookies: &CookieJar = req.cookies();
        if let Some(jwt_cookie) = cookies.get("jwt") {
            let token = jwt_cookie.value();
            let validation = Validation::new(Algorithm::HS256);

            if let Ok(token_data_claims) = decode::<Claims>(
                token,
                &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
                &validation,
            ) {
                let claims = token_data_claims.claims;
                return Outcome::Success(AuthGuard { claims });
            } else {
                warn!("JWT validation failed.");
            }
        }

        Outcome::Forward(Status::Unauthorized)
    }
}

pub struct ReadOnlyModeGuard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ReadOnlyModeGuard {
    type Error = ();

    async fn from_request(_req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if PUBLIC_CONFIG.read_only_mode {
            return Outcome::Error((Status::InternalServerError, ()));
        }

        Outcome::Success(ReadOnlyModeGuard)
    }
}
