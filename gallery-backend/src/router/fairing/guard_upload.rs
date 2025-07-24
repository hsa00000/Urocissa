use anyhow::Error;
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::router::AppError;

use super::VALIDATION;
use super::auth_utils::{try_authorize_upload_via_share, try_jwt_cookie_auth};

pub struct GuardUpload;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardUpload {
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try to authorize upload via share first
        if try_authorize_upload_via_share(req) {
            return Outcome::Success(GuardUpload);
        }

        // Fall back to JWT cookie authentication
        match try_jwt_cookie_auth(req, &VALIDATION) {
            Ok(_) => return Outcome::Success(GuardUpload),
            Err(err) => {
                let full_err = err.context("Authentication error");
                Outcome::Error((Status::Unauthorized, full_err))
            }
        }
    }
}
