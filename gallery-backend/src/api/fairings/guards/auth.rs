use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::api::GuardError;

use crate::api::fairings::VALIDATION;
use crate::api::fairings::utils::try_jwt_cookie_auth;

pub struct GuardAuth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardAuth {
    type Error = GuardError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match try_jwt_cookie_auth(req, &VALIDATION) {
            Ok(_) => Outcome::Success(GuardAuth),
            Err(err) => Outcome::Error((
                Status::InternalServerError,
                err.context("Authentication error").into(),
            )),
        }
    }
}
