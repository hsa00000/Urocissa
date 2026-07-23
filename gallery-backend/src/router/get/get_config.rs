// src/router/get/get_config.rs

use rocket::get;
use rocket::http::ContentType;
use rocket::serde::json::Json;

// Import PublicConfig
use crate::public::structure::config::{APP_CONFIG, PublicConfig};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_share::GuardShare;
use serde::Serialize;

use crate::router::{AppResult, GuardResult};
// use crate::public::error::{AppError, ErrorKind, ResultExt};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicConfigResponse {
    #[serde(flatten)]
    pub public: PublicConfig,
    pub has_password: bool,
    pub has_discord_hook: bool,
    pub has_auth_key: bool,
}

#[get("/get/config")]
pub fn get_config_handler(auth: GuardResult<GuardShare>) -> AppResult<Json<PublicConfigResponse>> {
    let _ = auth?;
    // Only return the public part
    let config = APP_CONFIG.get().unwrap().read().unwrap();
    let response = PublicConfigResponse {
        public: config.public.clone(),
        has_password: config.private.password.is_some(),
        has_discord_hook: config.private.discord_hook_url.is_some(),
        has_auth_key: config.private.auth_key.is_some(),
    };
    Ok(Json(response))
}

#[get("/get/config/export")]
pub fn export_config_handler(auth: GuardResult<GuardAuth>) -> AppResult<(ContentType, String)> {
    let _ = auth?;
    // Export remains full config for backup purposes
    let config = APP_CONFIG.get().unwrap().read().unwrap();
    let json = serde_json::to_string_pretty(&*config).unwrap_or_default();
    Ok((ContentType::JSON, json))
}

#[cfg(test)]
mod tests {
    use super::PublicConfigResponse;
    use crate::public::structure::config::AppConfig;

    #[test]
    fn public_response_does_not_serialize_saved_searches() {
        let config = AppConfig::default();
        let response = PublicConfigResponse {
            public: config.public,
            has_password: false,
            has_discord_hook: false,
            has_auth_key: false,
        };

        let value = serde_json::to_value(response).unwrap();
        assert!(value.get("savedSearches").is_none());
        assert!(value.get("private").is_none());
    }
}
