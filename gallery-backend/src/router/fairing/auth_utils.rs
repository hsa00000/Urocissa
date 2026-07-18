// src/router/fairing/auth_utils.rs
use crate::public::db::tree::TREE;
use crate::public::db::write_behind::WRITE_BEHIND;
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::{ResolvedShare, Share};
use crate::public::structure::config::APP_CONFIG;
use crate::router::claims::claims::Claims;
use anyhow::{Error, Result, anyhow};
use arrayvec::ArrayString;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, Validation, decode};
use log::info;
use rocket::Request;
use serde::de::DeserializeOwned;

// Error types for share validation are now handled by AppError

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
    let secret_key = APP_CONFIG
        .get()
        .unwrap()
        .read()
        .unwrap()
        .get_jwt_secret_key();

    match decode::<T>(token, &DecodingKey::from_secret(&secret_key), validation) {
        Ok(token_data) => Ok(token_data.claims),
        Err(err) => {
            info!("Token decode failed: {err:?}");
            Err(Error::from(err).context("Failed to decode JWT token"))
        }
    }
}

/// Try to authenticate via JWT cookie and check if user is admin
pub fn try_jwt_cookie_auth(req: &Request<'_>, validation: &Validation) -> Result<Claims> {
    // If no password is set, allow access as admin
    if APP_CONFIG
        .get()
        .unwrap()
        .read()
        .unwrap()
        .private
        .password
        .is_none()
    {
        return Ok(Claims::new_admin());
    }

    if let Some(jwt_cookie) = req.cookies().get("jwt") {
        let token = jwt_cookie.value();
        let claims = my_decode_token::<Claims>(token, validation)?;
        if claims.is_admin() {
            return Ok(claims);
        }
        return Err(anyhow!("User is not an admin"));
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
fn validate_share_access(share: &Share, req: &Request<'_>) -> Result<(), AppError> {
    // 1. Check expiration
    if share.exp > 0 {
        let now = Utc::now().timestamp_millis() / 1000;

        if now > share.exp {
            return Err(AppError::new(
                ErrorKind::PermissionDenied,
                "Share link expired",
            ));
        }
    }

    // 2. Check password
    if let Some(ref pwd) = share.password {
        // Check Header: x-share-password
        if let Some(header_pwd) = req.headers().get_one("x-share-password")
            && header_pwd == pwd
        {
            return Ok(());
        }

        return Err(AppError::new(
            ErrorKind::Auth,
            "Share password required or incorrect",
        ));
    }

    Ok(())
}

fn resolve_share_internal(
    album_id: &str,
    share_id: &str,
    req: &Request<'_>,
) -> Result<Option<Claims>, AppError> {
    let durable = TREE
        .store
        .read(|table| {
            table
                .get(album_id)
                .map(|value| value.map(|value| value.value()))
        })
        .map_err(|e| AppError::from_err(ErrorKind::Database, e))?;
    let abstract_data = WRITE_BEHIND
        .logical_record(album_id, durable)
        .ok_or_else(|| {
            AppError::new(
                ErrorKind::NotFound,
                format!("Album not found for id '{album_id}'"),
            )
        })?;

    let AbstractData::Album(mut album) = abstract_data else {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("Data with id '{album_id}' is not an album"),
        ));
    };

    let share = album.metadata.share_list.remove(share_id).ok_or_else(|| {
        AppError::new(
            ErrorKind::NotFound,
            format!("Share '{share_id}' not found in album '{album_id}'"),
        )
    })?;

    // Validate share access (password and expiration)
    validate_share_access(&share, req)?;

    let resolved_share = ResolvedShare::new(
        ArrayString::<64>::from(album_id)
            .map_err(|_| AppError::new(ErrorKind::Internal, "Failed to parse album_id"))?,
        album.metadata.title,
        share,
    );
    let claims = Claims::new_share(resolved_share);
    Ok(Some(claims))
}

/// Try to resolve album and share from headers
pub fn try_resolve_share_from_headers(req: &Request<'_>) -> Result<Option<Claims>, AppError> {
    let album_id = req.headers().get_one("x-album-id");
    let share_id = req.headers().get_one("x-share-id");

    match (album_id, share_id) {
        (None, None) => Ok(None),

        (Some(_), None) | (None, Some(_)) => Err(AppError::new(
            ErrorKind::InvalidInput,
            "Both x-album-id and x-share-id must be provided together",
        )),

        (Some(album_id), Some(share_id)) => resolve_share_internal(album_id, share_id, req),
    }
}

/// Try to resolve album and share from query parameters
pub fn try_resolve_share_from_query(req: &Request<'_>) -> Result<Option<Claims>, AppError> {
    let album_id = req.query_value::<&str>("albumId").and_then(Result::ok);
    let share_id = req.query_value::<&str>("shareId").and_then(Result::ok);

    match (album_id, share_id) {
        (None, None) => Ok(None),

        (Some(_), None) | (None, Some(_)) => Err(AppError::new(
            ErrorKind::InvalidInput,
            "Both albumId and shareId must be provided together",
        )),

        (Some(album_id), Some(share_id)) => resolve_share_internal(album_id, share_id, req),
    }
}

/// Try to authorize upload via share headers with upload permission
pub fn try_authorize_upload_via_share(req: &Request<'_>) -> bool {
    if let Some(album_id) = req.headers().get_one("x-album-id")
        && let Some(share_id) = req.headers().get_one("x-share-id")
        && let Ok(durable) = TREE.store.read(|table| {
            table
                .get(album_id)
                .map(|value| value.map(|value| value.value()))
        })
        && let Some(AbstractData::Album(mut album)) = WRITE_BEHIND.logical_record(album_id, durable)
        && let Some(share) = album.metadata.share_list.remove(share_id)
        && share.show_upload
        && validate_share_access(&share, req).is_ok()
        && let Some(Ok(album_id_parsed)) = req.query_value::<&str>("presigned_album_id_opt")
    {
        return album.object.id.as_str() == album_id_parsed;
    }

    false
}
