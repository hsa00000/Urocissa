use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::api::fairings::VALIDATION;
use crate::api::fairings::utils::{
    try_jwt_cookie_auth, try_resolve_share_from_headers, try_resolve_share_from_query,
};
use crate::api::GuardError;
use crate::api::claims::types::Claims;

pub struct GuardShare {
    pub claims: Claims,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardShare {
    type Error = GuardError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // headers
        match try_resolve_share_from_headers(req) {
            Ok(Some(claims)) => return Outcome::Success(GuardShare { claims }),
            Ok(None) => {}
            Err(err) => {
                return Outcome::Error((
                    Status::InternalServerError,
                    err.context("Header share resolution failed").into(),
                ));
            }
        }

        // query
        match try_resolve_share_from_query(req) {
            Ok(Some(claims)) => return Outcome::Success(GuardShare { claims }),
            Ok(None) => {}
            Err(err) => {
                return Outcome::Error((
                    Status::InternalServerError,
                    err.context("Query share resolution failed").into(),
                ));
            }
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
