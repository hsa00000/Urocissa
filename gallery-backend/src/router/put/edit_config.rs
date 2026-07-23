use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use log::error;
use rocket::http::Status;
use rocket::put;
use rocket::serde::json::Json;
use serde::Deserialize;
use tokio::task::spawn_blocking;

use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::config::{AppConfig, WriteBehindConfig};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};

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
        AppConfig::mutate(|current_config| {
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

            if let Some(key) = req_data.auth_key {
                let trimmed = key.trim();
                current_config.private.auth_key = (!trimmed.is_empty()).then(|| trimmed.to_owned());
            }

            if let Some(hook) = req_data.discord_hook_url {
                let trimmed = hook.trim();
                current_config.private.discord_hook_url =
                    (!trimmed.is_empty()).then(|| trimmed.to_owned());
            }

            current_config
                .public
                .write_behind
                .validate()
                .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))?;
            Ok(())
        })
        .map_err(|error| {
            error!("Failed to update config: {error}");
            error
        })?;

        crate::tasks::batcher::start_watcher::reload_watcher();
        crate::public::db::write_behind::WRITE_BEHIND.config_updated();
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
        AppConfig::mutate(|current_config| {
            if req_data.old_password != current_config.private.password {
                // HTTP 400 avoids the global 401 session-expired redirect.
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "Incorrect current password",
                ));
            }

            current_config.private.password = req_data.password.and_then(|password| {
                let password = password.trim();
                (!password.is_empty()).then(|| password.to_owned())
            });
            Ok(())
        })
        .map_err(|error| {
            error!("Failed to update password: {error}");
            error
        })?;

        Ok(Status::Ok)
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Task join error"))?
}
