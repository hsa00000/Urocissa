use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use super::VALIDATION;
use super::auth_utils::{try_authorize_upload_via_share, try_jwt_cookie_auth};

pub struct GuardUpload;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardUpload {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try to authorize upload via share first
        if try_authorize_upload_via_share(req) {
            return Outcome::Success(GuardUpload);
        }

        // Fall back to JWT cookie authentication
        if try_jwt_cookie_auth(req, &VALIDATION).is_some() {
            return Outcome::Success(GuardUpload);
        }

        Outcome::Forward(Status::Unauthorized)
    }
}
