use crate::public::structure::config::APP_CONFIG;
use anyhow::Error;
use anyhow::anyhow;
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

pub struct GuardReadOnlyMode;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardReadOnlyMode {
    type Error = Error;
    async fn from_request(_req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if APP_CONFIG.get().unwrap().read().unwrap().read_only_mode {
            return Outcome::Error((
                Status::InternalServerError,
                anyhow!("Read-only mode is enabled").into(),
            ));
        }

        Outcome::Success(GuardReadOnlyMode)
    }
}
