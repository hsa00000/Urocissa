use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::router::claims::claims::Claims;

use super::VALIDATION;
use super::auth_utils::{
    try_jwt_cookie_auth, try_resolve_share_from_headers, try_resolve_share_from_query,
};

pub struct GuardShare {
    pub claims: Claims,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardShare {
    type Error = ();

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
        if let Some(claims) = try_jwt_cookie_auth(req, &VALIDATION) {
            return Outcome::Success(GuardShare { claims });
        }

        Outcome::Error((Status::Unauthorized, ()))
    }
}
