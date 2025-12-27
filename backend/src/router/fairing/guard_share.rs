use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use super::VALIDATION;
use super::auth_utils::{
    ShareError, try_jwt_cookie_auth, try_resolve_share_from_headers, try_resolve_share_from_query,
};
use crate::router::GuardError;
use crate::router::claims::claims::Claims;

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
            Ok(None) => {} // No share headers, continue
            Err(err) => {
                let status = match err {
                    ShareError::Unauthorized => Status::Unauthorized,
                    ShareError::Expired => Status::Forbidden,
                    ShareError::Internal(_) => Status::InternalServerError,
                };

                let err_msg = match err {
                    ShareError::Internal(e) => e,
                    _ => anyhow::anyhow!("Share authentication failed: {:?}", err),
                };

                return Outcome::Error((
                    status,
                    GuardError {
                        status,
                        error: err_msg,
                    },
                ));
            }
        }

        // query
        match try_resolve_share_from_query(req) {
            Ok(Some(claims)) => return Outcome::Success(GuardShare { claims }),
            Ok(None) => {}
            Err(err) => {
                let status = match err {
                    ShareError::Unauthorized => Status::Unauthorized,
                    ShareError::Expired => Status::Forbidden,
                    ShareError::Internal(_) => Status::InternalServerError,
                };

                let err_msg = match err {
                    ShareError::Internal(e) => e,
                    _ => anyhow::anyhow!("Share authentication failed: {:?}", err),
                };

                return Outcome::Error((
                    status,
                    GuardError {
                        status,
                        error: err_msg,
                    },
                ));
            }
        }

        // Fall back to JWT cookie authentication (Admin)
        match try_jwt_cookie_auth(req, &VALIDATION) {
            Ok(claims) => return Outcome::Success(GuardShare { claims }),
            Err(err) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    GuardError {
                        status: Status::Unauthorized,
                        error: err.context("Authentication error"),
                    },
                ));
            }
        }
    }
}
