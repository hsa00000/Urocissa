use crate::public::structure::album::{Album, ResolvedShare};
use crate::router::claims::claims::Claims;
use crate::router::post::authenticate::JSON_WEB_TOKEN_SECRET_KEY;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use arrayvec::ArrayString;
use jsonwebtoken::{DecodingKey, Validation, decode};
use rocket::Request;
use serde::de::DeserializeOwned;
/// Extract and validate Authorization header Bearer token
pub fn extract_bearer_token<'a>(req: &'a Request<'_>) -> Result<&'a str> {
    let auth_header = match req.headers().get_one("Authorization") {
        Some(header) => header,
        None => {
            return Err(anyhow!("Request is missing the Authorization header"));
        }
    };

    match auth_header.strip_prefix("Bearer ") {
        Some(token) => Ok(token),
        None => {
            return Err(anyhow!(
                "Authorization header format is invalid, expected 'Bearer <token>'"
            ));
        }
    }
}

/// Decode JWT token with given claims type and validation
pub fn my_decode_token<T: DeserializeOwned>(token: &str, validation: &Validation) -> Result<T> {
    match decode::<T>(
        token,
        &DecodingKey::from_secret(&*JSON_WEB_TOKEN_SECRET_KEY),
        validation,
    ) {
        Ok(token_data) => Ok(token_data.claims),
        Err(err) => {
            return Err(Error::from(err).context("Failed to decode JWT token"));
        }
    }
}

/// Try to authenticate via JWT cookie and check if user is admin
pub fn try_jwt_cookie_auth(req: &Request<'_>, validation: &Validation) -> Result<Claims> {
    if let Some(jwt_cookie) = req.cookies().get("jwt") {
        let token = jwt_cookie.value();
        let claims = my_decode_token::<Claims>(token, validation)?;
        if claims.is_admin() {
            return Ok(claims);
        } else {
            return Err(anyhow!("User is not an admin"));
        }
    }
    Err(anyhow!("JWT not found in cookies"))
}

/// Extract hash from the request URL path (last segment before extension)
pub fn extract_hash_from_path(req: &Request<'_>) -> Result<String> {
    let hash_opt = req
        .uri()
        .path()
        .segments()
        .last()
        .and_then(|hash_with_ext| hash_with_ext.rsplit_once('.'))
        .map(|(hash, _ext)| hash.to_string());

    match hash_opt {
        Some(hash) => Ok(hash),
        None => Err(anyhow!("No valid 'hash' parameter found in the uri")),
    }
}

/// Try to resolve album and share from headers
pub fn try_resolve_share_from_headers(req: &Request<'_>) -> Result<Option<Claims>> {
    let album_id = req.headers().get_one("x-album-id");
    let share_id = req.headers().get_one("x-share-id");

    match (album_id, share_id) {
        (None, None) => Ok(None),

        (Some(_), None) | (None, Some(_)) => Err(anyhow!(
            "Both x-album-id and x-share-id must be provided together"
        )),

        (Some(album_id), Some(share_id)) => {
            let conn = crate::public::db::sqlite::DB_POOL.get().map_err(|_| anyhow!("Failed to open database"))?;
            let album: Album = conn.query_row(
                "SELECT * FROM album WHERE id = ?",
                [album_id],
                |row| Album::from_row(row)
            ).map_err(|_| anyhow!("Album not found for id '{}'", album_id))?;

            let share = album.share_list.get(share_id)
                .ok_or_else(|| anyhow!("Share '{}' not found in album '{}'", share_id, album_id))?
                .clone();

            let resolved_share = ResolvedShare::new(
                ArrayString::<64>::from(album_id)
                    .map_err(|_| anyhow!("Failed to parse album_id"))?,
                album.title,
                share,
            );
            let claims = Claims::new_share(resolved_share);
            Ok(Some(claims))
        }
    }
}

/// Try to resolve album and share from query parameters
pub fn try_resolve_share_from_query(req: &Request<'_>) -> Result<Option<Claims>> {
    let album_id = req.query_value::<&str>("albumId").and_then(Result::ok);
    let share_id = req.query_value::<&str>("shareId").and_then(Result::ok);

    match (album_id, share_id) {
        (None, None) => Ok(None),

        (Some(_), None) | (None, Some(_)) => Err(anyhow!(
            "Both albumId and shareId must be provided together"
        )),

        (Some(album_id), Some(share_id)) => {
            let conn = crate::public::db::sqlite::DB_POOL.get().map_err(|_| anyhow!("Failed to open database"))?;
            let album: Album = conn.query_row(
                "SELECT * FROM album WHERE id = ?",
                [album_id],
                |row| Album::from_row(row)
            ).map_err(|_| anyhow!("Album not found for id '{}'", album_id))?;

            let share = album.share_list.get(share_id)
                .ok_or_else(|| anyhow!("Share '{}' not found in album '{}'", share_id, album_id))?
                .clone();

            let resolved_share = ResolvedShare::new(
                ArrayString::<64>::from(album_id)
                    .map_err(|_| anyhow!("Failed to parse album_id"))?,
                album.title,
                share,
            );
            let claims = Claims::new_share(resolved_share);
            Ok(Some(claims))
        }
    }
}

/// Try to authorize upload via share headers with upload permission
pub fn try_authorize_upload_via_share(req: &Request<'_>) -> bool {
    let album_id = req.headers().get_one("x-album-id");
    let share_id = req.headers().get_one("x-share-id");

    if let (Some(album_id), Some(share_id)) = (album_id, share_id) {
        if let Ok(conn) = crate::public::db::sqlite::DB_POOL.get() {
            if let Ok(album) = conn.query_row(
                "SELECT * FROM album WHERE id = ?",
                [album_id],
                |row| Album::from_row(row)
            ) {
                if let Some(share) = album.share_list.get(share_id) {
                    if share.show_upload {
                        if let Some(Ok(album_id_parsed)) =
                            req.query_value::<&str>("presigned_album_id_opt")
                        {
                            return album.id.as_str() == album_id_parsed;
                        }
                    }
                }
            }
        }
    }
    false
}
