use crate::public::constant::redb::DATA_TABLE;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::{ResolvedShare, Share};
// 引入 APP_CONFIG
use crate::public::structure::config::APP_CONFIG;
use crate::router::claims::claims::Claims;
// 移除舊的 get_jwt_secret_key 引入
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use arrayvec::ArrayString;
use jsonwebtoken::{DecodingKey, Validation, decode};
use log::info;
use redb::ReadableDatabase;
use rocket::Request;
use serde::de::DeserializeOwned;
use std::time::{SystemTime, UNIX_EPOCH};

// Error types for share validation
#[derive(Debug)]
pub enum ShareError {
    Expired,
    Unauthorized,
    Internal(anyhow::Error),
}

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
    // 獲取 Secret Key
    let secret_key = APP_CONFIG.get().unwrap().read().unwrap().get_jwt_secret_key();

    match decode::<T>(
        token,
        &DecodingKey::from_secret(&secret_key),
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

/// Validate share access: check expiration and password
fn validate_share_access(share: &Share, req: &Request<'_>) -> Result<(), ShareError> {
    // 1. Check expiration
    if share.exp > 0 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ShareError::Internal(anyhow!("Time error: {}", e)))?
            .as_secs();

        let distance = share.exp.saturating_sub(now);
        info!("Expire 壽命距離現在時間是剩下 {} 秒", distance);

        if now > share.exp {
            return Err(ShareError::Expired);
        }
    }

    // 2. Check password
    if let Some(ref pwd) = share.password {
        // Check Header: x-share-password
        if let Some(header_pwd) = req.headers().get_one("x-share-password") {
            if header_pwd == pwd {
                return Ok(());
            }
        }

        // Check Query Param: password
        if let Some(Ok(query_pwd)) = req.query_value::<&str>("password") {
            if query_pwd == pwd {
                return Ok(());
            }
        }

        return Err(ShareError::Unauthorized);
    }

    Ok(())
}

/// Try to resolve album and share from headers
pub fn try_resolve_share_from_headers(req: &Request<'_>) -> Result<Option<Claims>, ShareError> {
    let album_id = req.headers().get_one("x-album-id");
    let share_id = req.headers().get_one("x-share-id");

    match (album_id, share_id) {
        (None, None) => Ok(None),

        (Some(_), None) | (None, Some(_)) => Err(ShareError::Internal(anyhow!(
            "Both x-album-id and x-share-id must be provided together"
        ))),

        (Some(album_id), Some(share_id)) => {
            let read_txn = TREE.in_disk.begin_read().map_err(|e| {
                ShareError::Internal(anyhow!("Failed to begin read transaction: {}", e))
            })?;

            let table = read_txn
                .open_table(DATA_TABLE)
                .map_err(|e| ShareError::Internal(anyhow!("Failed to open data table: {}", e)))?;

            let data_guard = table
                .get(album_id)
                .map_err(|e| ShareError::Internal(anyhow!("Failed to get data from table: {}", e)))?
                .ok_or_else(|| {
                    ShareError::Internal(anyhow!("Album not found for id '{}'", album_id))
                })?;

            let abstract_data = data_guard.value();
            let mut album = match abstract_data {
                AbstractData::Album(album) => album,
                _ => {
                    return Err(ShareError::Internal(anyhow!(
                        "Data with id '{}' is not an album",
                        album_id
                    )));
                }
            };

            let share = album.metadata.share_list.remove(share_id).ok_or_else(|| {
                ShareError::Internal(anyhow!(
                    "Share '{}' not found in album '{}'",
                    share_id,
                    album_id
                ))
            })?;

            // Validate share access (password and expiration)
            validate_share_access(&share, req)?;

            let resolved_share = ResolvedShare::new(
                ArrayString::<64>::from(album_id)
                    .map_err(|_| ShareError::Internal(anyhow!("Failed to parse album_id")))?,
                album.metadata.title,
                share,
            );
            let claims = Claims::new_share(resolved_share);
            Ok(Some(claims))
        }
    }
}

/// Try to resolve album and share from query parameters
pub fn try_resolve_share_from_query(req: &Request<'_>) -> Result<Option<Claims>, ShareError> {
    let album_id = req.query_value::<&str>("albumId").and_then(Result::ok);
    let share_id = req.query_value::<&str>("shareId").and_then(Result::ok);

    match (album_id, share_id) {
        (None, None) => Ok(None),

        (Some(_), None) | (None, Some(_)) => Err(ShareError::Internal(anyhow!(
            "Both albumId and shareId must be provided together"
        ))),

        (Some(album_id), Some(share_id)) => {
            let read_txn = TREE.in_disk.begin_read().map_err(|e| {
                ShareError::Internal(anyhow!("Failed to begin read transaction: {}", e))
            })?;

            let table = read_txn
                .open_table(DATA_TABLE)
                .map_err(|e| ShareError::Internal(anyhow!("Failed to open data table: {}", e)))?;

            let data_guard = table
                .get(album_id)
                .map_err(|e| ShareError::Internal(anyhow!("Failed to get data from table: {}", e)))?
                .ok_or_else(|| {
                    ShareError::Internal(anyhow!("Album not found for id '{}'", album_id))
                })?;

            let abstract_data = data_guard.value();
            let mut album = match abstract_data {
                AbstractData::Album(album) => album,
                _ => {
                    return Err(ShareError::Internal(anyhow!(
                        "Data with id '{}' is not an album",
                        album_id
                    )));
                }
            };

            let share = album.metadata.share_list.remove(share_id).ok_or_else(|| {
                ShareError::Internal(anyhow!(
                    "Share '{}' not found in album '{}'",
                    share_id,
                    album_id
                ))
            })?;

            // Validate share access (password and expiration)
            validate_share_access(&share, req)?;

            let resolved_share = ResolvedShare::new(
                ArrayString::<64>::from(album_id)
                    .map_err(|_| ShareError::Internal(anyhow!("Failed to parse album_id")))?,
                album.metadata.title,
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
        if let Ok(read_txn) = TREE.in_disk.begin_read() {
            if let Ok(table) = read_txn.open_table(DATA_TABLE) {
                if let Ok(Some(data_guard)) = table.get(album_id) {
                    let abstract_data = data_guard.value();
                    if let AbstractData::Album(mut album) = abstract_data {
                        if let Some(share) = album.metadata.share_list.remove(share_id) {
                            if share.show_upload {
                                // Ensure password and expiration are also valid for upload
                                if validate_share_access(&share, req).is_err() {
                                    return false;
                                }

                                if let Some(Ok(album_id_parsed)) =
                                    req.query_value::<&str>("presigned_album_id_opt")
                                {
                                    return album.object.id.as_str() == album_id_parsed;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
