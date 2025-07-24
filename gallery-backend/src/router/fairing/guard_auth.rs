use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use super::VALIDATION;
use super::auth_utils::try_jwt_cookie_auth;

pub struct GuardAuth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardAuth {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if try_jwt_cookie_auth(req, &VALIDATION).is_some() {
            Outcome::Success(GuardAuth)
        } else {
            Outcome::Error((Status::Unauthorized, ()))
        }
    }
}
