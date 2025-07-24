use jsonwebtoken::{DecodingKey, decode};
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};

use crate::public::constant::redb::ALBUM_TABLE;
use crate::public::db::tree::TREE;
use crate::router::claims::claims::Claims;
use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;

use super::VALIDATION;

pub struct GuardUpload;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GuardUpload {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let album_id = req.headers().get_one("x-album-id");
        let share_id = req.headers().get_one("x-share-id");

        if let (Some(album_id), Some(share_id)) = (album_id, share_id) {
            let read_txn = TREE.in_disk.begin_read().unwrap();
            let table = read_txn.open_table(ALBUM_TABLE).unwrap();

            if let Some(album_guard) = table.get(album_id).unwrap() {
                let mut album = album_guard.value();

                if let Some(share) = album.share_list.remove(share_id)
                    && share.show_upload
                    && let Some(Ok(album_id_parsed)) =
                        req.query_value::<&str>("presigned_album_id_opt")
                    && album.id.as_str() == album_id_parsed
                {
                    return Outcome::Success(GuardUpload);
                } else {
                    return Outcome::Forward(Status::Unauthorized);
                }
            }
        }

        if let Some(jwt_cookie) = req.cookies().get("jwt") {
            let token = jwt_cookie.value();
            match decode::<Claims>(
                token,
                &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
                &VALIDATION,
            ) {
                Ok(token_data) if token_data.claims.is_admin() => {
                    return Outcome::Success(GuardUpload);
                }
                _ => {
                    warn!("JWT validation failed");
                }
            }
        }

        Outcome::Forward(Status::Unauthorized)
    }
}
