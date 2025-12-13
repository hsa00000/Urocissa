use crate::api::claims::types::Claims;
use crate::api::handlers::auth::JSON_WEB_TOKEN_SECRET_KEY;
use crate::database::ops::tree::TREE;
use crate::database::schema::relations::album_share::{ResolvedShare, Share};
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use arrayvec::ArrayString;
use jsonwebtoken::{DecodingKey, Validation, decode};
use log::info;
use rocket::Request;
use serde::de::DeserializeOwned;
use std::time::{SystemTime, UNIX_EPOCH};

// 1. 定義錯誤類型 (不依賴 thiserror)
#[derive(Debug)]
pub enum ShareError {
    Expired,
    Unauthorized,
    Internal(anyhow::Error), // 用於包裹資料庫或其他系統錯誤
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
pub fn my_decode_token<T: DeserializeOwned>(
    token: impl AsRef<str>,
    validation: &Validation,
) -> Result<T> {
    let token = token.as_ref();
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

fn resolve_share_from_db(
    album_id: impl AsRef<str>,
    share_id: impl AsRef<str>,
) -> Result<ResolvedShare> {
    let album_id = album_id.as_ref();
    let share_id = share_id.as_ref();
    let txn = TREE
        .begin_read()
        .map_err(|e| anyhow!("DB read error: {}", e))?;

    // Get share from album_share table
    let share_table =
        txn.open_table(crate::database::schema::relations::album_share::ALBUM_SHARE_TABLE)?;
    let share_key = (album_id, share_id);
    let share_bytes = share_table
        .get(share_key)?
        .ok_or_else(|| anyhow!("Share '{}' not found in album '{}'", share_id, album_id))?;
    let share: Share = bitcode::decode(share_bytes.value())?;

    // Get album title from meta_album table
    let meta_table = txn.open_table(crate::database::schema::meta_album::META_ALBUM_TABLE)?;
    let album_bytes = meta_table
        .get(album_id)?
        .ok_or_else(|| anyhow!("Album '{}' not found", album_id))?;
    let album: crate::database::schema::meta_album::AlbumMetadataSchema =
        bitcode::decode(album_bytes.value())?;
    let album_title = album.title;

    let resolved_share = ResolvedShare {
        album_id: ArrayString::<64>::from(album_id)
            .map_err(|_| anyhow!("Failed to parse album_id"))?,
        album_title,
        share,
    };

    Ok(resolved_share)
}

// 2. 修改 validate_share_access 回傳 ShareError
fn validate_share_access(share: &Share, req: &Request<'_>) -> Result<(), ShareError> {
    // 1. 檢查過期 (Expiration)
    if share.exp > 0 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ShareError::Internal(anyhow!("Time error: {}", e)))?
            .as_secs();

        // --- 新增測試用 Log 區塊 (檢查剩餘時間) ---
        // 使用 saturating_sub 避免時間差微小誤差導致 panic
        let distance = share.exp.saturating_sub(now);
        info!("Expire 壽命距離現在時間是剩下 {} 秒", distance);
        // ----------------------------------------

        if now > share.exp {
            return Err(ShareError::Expired); // 明確的過期錯誤
        }
    }

    // 2. 檢查密碼 (Password)
    if let Some(ref pwd) = share.password {
        // 優先檢查 Header: x-share-password
        if let Some(header_pwd) = req.headers().get_one("x-share-password") {
            if header_pwd == pwd {
                return Ok(());
            }
        }

        // 其次檢查 Query Param: password (方便直接連結訪問)
        if let Some(Ok(query_pwd)) = req.query_value::<&str>("password") {
            if query_pwd == pwd {
                return Ok(());
            }
        }

        return Err(ShareError::Unauthorized); // 明確的權限錯誤
    }

    Ok(())
}

/// Try to resolve album and share from headers
pub fn try_resolve_share_from_headers(req: &Request<'_>) -> Result<Option<Claims>, ShareError> {
    let album_id = req.headers().get_one("x-album-id");
    let share_id = req.headers().get_one("x-share-id");

    match (album_id, share_id) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            // 這種參數錯誤視為 Internal 或直接 Unauthorized 視需求而定，這裡用 Internal 簡化
            Err(ShareError::Internal(anyhow!(
                "Both x-album-id and x-share-id must be provided"
            )))
        }
        (Some(album_id), Some(share_id)) => {
            // 資料庫錯誤包裝成 Internal
            let resolved =
                resolve_share_from_db(album_id, share_id).map_err(ShareError::Internal)?;

            validate_share_access(&resolved.share, req)?;

            Ok(Some(Claims::new_share(resolved)))
        }
    }
}

/// Try to resolve album and share from query parameters
pub fn try_resolve_share_from_query(req: &Request<'_>) -> Result<Option<Claims>, ShareError> {
    let album_id = req.query_value::<&str>("albumId").and_then(Result::ok);
    let share_id = req.query_value::<&str>("shareId").and_then(Result::ok);

    match (album_id, share_id) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            // 這種參數錯誤視為 Internal 或直接 Unauthorized 視需求而定，這裡用 Internal 簡化
            Err(ShareError::Internal(anyhow!(
                "Both albumId and shareId must be provided together"
            )))
        }
        (Some(album_id), Some(share_id)) => {
            // 資料庫錯誤包裝成 Internal
            let resolved =
                resolve_share_from_db(album_id, share_id).map_err(ShareError::Internal)?;

            validate_share_access(&resolved.share, req)?;

            Ok(Some(Claims::new_share(resolved)))
        }
    }
}

/// Try to authorize upload via share headers with upload permission
pub fn try_authorize_upload_via_share(req: &Request<'_>) -> bool {
    let album_id = req.headers().get_one("x-album-id");
    let share_id = req.headers().get_one("x-share-id");

    if let (Some(album_id), Some(share_id)) = (album_id, share_id) {
        if let Ok(txn) = TREE.begin_read() {
            if let Ok(share_table) =
                txn.open_table(crate::database::schema::relations::album_share::ALBUM_SHARE_TABLE)
            {
                if let Ok(Some(share_bytes)) = share_table.get((album_id, share_id)) {
                    if let Ok(share) = bitcode::decode::<Share>(share_bytes.value()) {
                        if share.show_upload {
                            // 確保即使允許上傳，使用者仍需通過密碼與過期時間檢查
                            if validate_share_access(&share, req).is_err() {
                                return false;
                            }

                            if let Some(Ok(album_id_parsed)) =
                                req.query_value::<&str>("presigned_album_id_opt")
                            {
                                return album_id == album_id_parsed;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
