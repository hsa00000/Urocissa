// src/router/put/edit_config.rs

use log::error;
use rocket::http::Status;
use rocket::put;
use rocket::serde::json::Json;
use tokio::task::spawn_blocking;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// Import PublicConfig
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::config::{APP_CONFIG, AppConfig, WriteBehindConfig};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialUpdateConfigRequest {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub limits: Option<HashMap<String, String>>,
    pub sync_paths: Option<HashSet<PathBuf>>,
    pub read_only_mode: Option<bool>,
    pub disable_img: Option<bool>,
    pub write_behind: Option<WriteBehindConfig>,
    pub auth_key: Option<String>,
    pub discord_hook_url: Option<String>,
}

#[put("/put/config", data = "<req>")]
pub async fn update_config_handler(
    _auth: GuardAuth,
    read_only: GuardResult<GuardReadOnlyMode>,
    req: Json<PartialUpdateConfigRequest>,
) -> AppResult<Status> {
    let _ = read_only?;
    let req_data = req.into_inner();

    spawn_blocking(move || -> Result<Status, AppError> {
        // 1. Get the current full config
        let mut current_config = {
            let read_lock = APP_CONFIG.get().unwrap().read().unwrap();
            read_lock.clone()
        };

        // 2. Apply updates to PublicConfig fields
        if let Some(address) = req_data.address {
            current_config.public.address = address;
        }
        if let Some(port) = req_data.port {
            current_config.public.port = port;
        }
        if let Some(limits) = req_data.limits {
            current_config.public.limits = limits;
        }
        if let Some(sync_paths) = req_data.sync_paths {
            current_config.public.sync_paths = sync_paths;
        }
        if let Some(read_only_mode) = req_data.read_only_mode {
            current_config.public.read_only_mode = read_only_mode;
        }
        if let Some(disable_img) = req_data.disable_img {
            current_config.public.disable_img = disable_img;
        }
        if let Some(write_behind) = req_data.write_behind {
            current_config.public.write_behind = write_behind;
        }

        // 3. Apply updates to PrivateConfig fields
        if let Some(key) = req_data.auth_key {
            let trimmed = key.trim();
            if trimmed.is_empty() {
                current_config.private.auth_key = None;
            } else {
                current_config.private.auth_key = Some(trimmed.to_string());
            }
        }

        if let Some(hook) = req_data.discord_hook_url {
            let trimmed = hook.trim();
            if trimmed.is_empty() {
                current_config.private.discord_hook_url = None;
            } else {
                current_config.private.discord_hook_url = Some(trimmed.to_string());
            }
        }

        current_config
            .public
            .write_behind
            .validate()
            .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))?;

        // 4. Update using the modified full config
        AppConfig::update(current_config).map_err(|e| {
            error!("Failed to update config: {e}");
            AppError::from_err(ErrorKind::Internal, e)
        })?;

        Ok(Status::Ok)
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Task join error"))?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePasswordRequest {
    pub password: Option<String>,
    pub old_password: Option<String>,
}

#[put("/put/config/password", data = "<req>")]
pub async fn update_password_handler(
    _auth: GuardAuth,
    read_only: GuardResult<GuardReadOnlyMode>,
    req: Json<UpdatePasswordRequest>,
) -> AppResult<Status> {
    let _ = read_only?;
    let req_data = req.into_inner();

    spawn_blocking(move || -> Result<Status, AppError> {
        // 1. Get current config
        let mut current_config = APP_CONFIG.get().unwrap().read().unwrap().clone();

        // 2. Verify old password
        if req_data.old_password != current_config.private.password {
            // Using ErrorKind::InvalidInput (HTTP 400) to prevent frontend redirect (which happens on 401)
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "Incorrect current password",
            ));
        }

        // 3. Update password
        if let Some(pwd) = req_data.password {
            let trimmed_pwd = pwd.trim().to_string();
            if trimmed_pwd.is_empty() {
                current_config.private.password = None;
            } else {
                current_config.private.password = Some(trimmed_pwd);
            }
        } else {
            // Explicitly requested to remove password?
            // If the frontend sends null/None, it usually means "don't change" or "remove"?
            // In our previous logic, we used empty string to remove.
            // Let's stick to: "If you call this endpoint, you are updating the password."
            // If `password` is None, we'll treat it as "Remove password" for safety/clarity,
            // OR we can decide based on frontend.
            // Let's assume frontend sends Some("") to remove.
            // If frontend sends None, we do nothing? No, this endpoint is specific for updating password.
            // Let's assume:
            // Some(pwd) -> update (or remove if empty)
            // None -> remove? Or Error?
            // Let's default to "None = Remove" to be consistent with "optional string".
            current_config.private.password = None;
        }

        // 4. Update
        AppConfig::update(current_config).map_err(|e| {
            error!("Failed to update config: {e}");
            AppError::from_err(ErrorKind::Internal, e)
        })?;

        Ok(Status::Ok)
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Task join error"))?
}
