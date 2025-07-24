use anyhow::Error;
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::public::config::PUBLIC_CONFIG;
use crate::router::GuardError;

pub struct GuardReadOnlyMode;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardReadOnlyMode {
    type Error = GuardError;

    async fn from_request(_req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if PUBLIC_CONFIG.read_only_mode {
            return Outcome::Error((
                Status::InternalServerError,
                Error::msg("Read-only mode is enabled").into(),
            ));
        }

        Outcome::Success(GuardReadOnlyMode)
    }
}
