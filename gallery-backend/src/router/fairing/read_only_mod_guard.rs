use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::public::config::PUBLIC_CONFIG;

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
