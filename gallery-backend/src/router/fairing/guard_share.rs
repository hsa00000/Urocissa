use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use super::VALIDATION;
use super::auth_utils::{
    try_jwt_cookie_auth, try_resolve_share_from_headers, try_resolve_share_from_query,
};
use crate::router::claims::claims::Claims;
use crate::router::GuardError;
use anyhow::Error;

pub struct GuardShare {
    pub claims: Claims,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardShare {
    type Error = GuardError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try to resolve share from headers first
        if let Some(claims) = try_resolve_share_from_headers(req) {
            return Outcome::Success(GuardShare { claims });
        }

        // Try to resolve share from query parameters
        if let Some(claims) = try_resolve_share_from_query(req) {
            return Outcome::Success(GuardShare { claims });
        }

        // Fall back to JWT cookie authentication
        match try_jwt_cookie_auth(req, &VALIDATION) {
            Ok(claims) => return Outcome::Success(GuardShare { claims }),
            Err(err) => {
                return Outcome::Error((
                    Status::InternalServerError,
                    err.context("Authentication error").into(),
                ));
            }
        }
    }
}
