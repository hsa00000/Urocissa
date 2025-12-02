use crate::public::db::tree::TREE;
use crate::table::relations::album_share::{ResolvedShare, Share};
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
    if let Some(auth_header) = req.headers().get_one("Authorization") {
        match auth_header.strip_prefix("Bearer ") {
            Some(token) => return Ok(token),
            None => {
                return Err(anyhow!(
                    "Authorization header format is invalid, expected 'Bearer <token>'"
                ));
            }
        }
    }

    if let Some(Ok(token)) = req.query_value::<&str>("token") {
        return Ok(token);
    }

    Err(anyhow!(
        "Request is missing the Authorization header or token query parameter"
    ))
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

fn resolve_share_from_db(album_id: &str, share_id: &str) -> Result<Claims> {
    let conn = TREE.get_connection().map_err(|e| anyhow!("DB connection error: {}", e))?;
    
    // Query both share details and album title in one go
    let sql = r#"
        SELECT 
            s.url, s.description, s.password, s.show_metadata, s.show_download, s.show_upload, s.exp,
            a.title
        FROM album_share s
        JOIN album a ON s.album_id = a.id
        WHERE s.album_id = ? AND s.url = ?
    "#;

    let (share, album_title): (Share, Option<String>) = conn.query_row(
        sql,
        [album_id, share_id],
        |row| {
            let url: String = row.get(0)?;
            let share = Share {
                url: ArrayString::from(&url).unwrap(),
                description: row.get(1)?,
                password: row.get(2)?,
                show_metadata: row.get(3)?,
                show_download: row.get(4)?,
                show_upload: row.get(5)?,
                exp: row.get(6)?,
            };
            let title: Option<String> = row.get(7)?;
            Ok((share, title))
        }
    ).map_err(|_| anyhow!("Share '{}' not found in album '{}'", share_id, album_id))?;

    let resolved_share = ResolvedShare::new(
        ArrayString::<64>::from(album_id)
            .map_err(|_| anyhow!("Failed to parse album_id"))?,
        album_title,
        share,
    );
    
    Ok(Claims::new_share(resolved_share))
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
            let claims = resolve_share_from_db(album_id, share_id)?;
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
            let claims = resolve_share_from_db(album_id, share_id)?;
            Ok(Some(claims))
        }
    }
}

/// Try to authorize upload via share headers with upload permission
pub fn try_authorize_upload_via_share(req: &Request<'_>) -> bool {
    let album_id = req.headers().get_one("x-album-id");
    let share_id = req.headers().get_one("x-share-id");

    if let (Some(album_id), Some(share_id)) = (album_id, share_id) {
        if let Ok(conn) = TREE.get_connection() {
            let show_upload: bool = conn.query_row(
                "SELECT show_upload FROM album_share WHERE album_id = ? AND url = ?",
                [album_id, share_id],
                |row| row.get(0)
            ).unwrap_or(false);

            if show_upload {
                if let Some(Ok(album_id_parsed)) =
                    req.query_value::<&str>("presigned_album_id_opt")
                {
                    return album_id == album_id_parsed;
                }
            }
        }
    }
    false
}
