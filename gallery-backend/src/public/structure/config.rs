// src/public/structure/config.rs

use anyhow::Context;
use base64::{Engine as _, engine::general_purpose};
use log::{info, warn};
use rand::{TryRng, rngs::SysRng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use crate::public::constant::storage::get_config_path;

pub static FALLBACK_SECRET_KEY: OnceLock<String> = OnceLock::new();

fn generate_secret_key() -> String {
    let mut secret = vec![0u8; 32];
    SysRng
        .try_fill_bytes(&mut secret)
        .expect("Failed to generate random secret key");
    general_purpose::STANDARD.encode(secret)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicConfig {
    pub address: String,
    pub port: u16,
    pub limits: HashMap<String, String>,
    pub sync_paths: HashSet<PathBuf>,
    pub read_only_mode: bool,
    pub disable_img: bool,
    #[serde(default)]
    pub write_behind: WriteBehindConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WriteBehindConfig {
    pub flush_interval_ms: u64,
    #[serde(rename = "softLimitMiB", alias = "softLimitMib")]
    pub soft_limit_mib: usize,
    #[serde(rename = "hardLimitMiB", alias = "hardLimitMib")]
    pub hard_limit_mib: usize,
}

impl Default for WriteBehindConfig {
    fn default() -> Self {
        Self {
            flush_interval_ms: 1_000,
            soft_limit_mib: 16,
            hard_limit_mib: 32,
        }
    }
}

impl WriteBehindConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (100..=60_000).contains(&self.flush_interval_ms),
            "writeBehind.flushIntervalMs must be between 100 and 60000"
        );
        anyhow::ensure!(
            self.soft_limit_mib > 0,
            "writeBehind.softLimitMiB must be positive"
        );
        anyhow::ensure!(
            self.soft_limit_mib < self.hard_limit_mib,
            "writeBehind.softLimitMiB must be smaller than hardLimitMiB"
        );
        anyhow::ensure!(
            self.hard_limit_mib <= 256,
            "writeBehind.hardLimitMiB must not exceed 256"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivateConfig {
    pub password: Option<String>,
    pub auth_key: Option<String>,
    pub discord_hook_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub public: PublicConfig,
    pub private: PrivateConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut limits = HashMap::new();
        limits.insert("json".to_string(), "10MiB".to_string());
        limits.insert("file".to_string(), "10GiB".to_string());
        limits.insert("data-form".to_string(), "10GiB".to_string());

        Self {
            public: PublicConfig {
                address: "0.0.0.0".to_string(),
                port: 5673,
                limits,
                sync_paths: HashSet::new(),
                read_only_mode: false,
                disable_img: false,
                write_behind: WriteBehindConfig::default(),
            },

            private: PrivateConfig {
                password: None,
                auth_key: None,
                discord_hook_url: None,
            },
        }
    }
}

pub static APP_CONFIG: OnceLock<RwLock<AppConfig>> = OnceLock::new();

impl AppConfig {
    pub fn get_jwt_secret_key(&self) -> Vec<u8> {
        match self.private.auth_key.as_ref() {
            Some(auth_key) => auth_key.as_bytes().to_vec(),
            None => FALLBACK_SECRET_KEY
                .get_or_init(generate_secret_key)
                .as_bytes()
                .to_vec(),
        }
    }

    pub fn init() {
        let config_path = get_config_path();
        let config_path_display = config_path.display();

        // Create default config file if it doesn't exist
        if !config_path.exists() {
            info!(
                "Configuration file not found at {config_path_display}. Creating default config.json..."
            );
            let default_config = AppConfig::default();

            if let Err(e) = Self::save_update(&default_config) {
                warn!("Failed to create default config file: {e}");
            } else {
                info!("Default configuration created successfully.");
            }
        }

        info!("Loading configuration from {config_path_display}");
        let (mut config, mut was_fallback) = Self::load_from_file();

        if let Err(error) = config.public.write_behind.validate() {
            warn!("Invalid write-behind configuration: {error}; using defaults");
            config.public.write_behind = WriteBehindConfig::default();
            was_fallback = true;
        }

        if was_fallback {
            info!("Overwriting invalid/empty config with defaults");
            if let Err(e) = Self::save_update(&config) {
                warn!("Failed to save default config: {e}");
            }
        }

        if config
            .private
            .auth_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .is_none()
        {
            config.private.auth_key = None;
            FALLBACK_SECRET_KEY.get_or_init(generate_secret_key);
        }

        APP_CONFIG
            .set(RwLock::new(config))
            .expect("Config already initialized");
    }

    fn load_from_file() -> (AppConfig, bool) {
        let config_path = get_config_path();
        let config_path_display = config_path.display();

        let file_content = fs::read_to_string(&config_path).unwrap_or_else(|e| {
            warn!("Failed to read config file {config_path_display}: {e}, using defaults");
            "{}".to_string()
        });

        match serde_json::from_str::<AppConfig>(&file_content) {
            Ok(config) => {
                info!("Successfully loaded configuration from {config_path_display}");
                (config, false)
            }
            Err(e) => {
                warn!(
                    "Failed to deserialize config from {config_path_display}: {e:?}, using defaults"
                );
                (AppConfig::default(), true)
            }
        }
    }

    pub fn update(mut new_config: AppConfig) -> anyhow::Result<()> {
        use crate::tasks::batcher::start_watcher::reload_watcher;

        info!("Updating configuration...");
        new_config.public.write_behind.validate()?;

        // Sanitize paths: only remove quotes and spaces, do not resolve paths
        let sanitized_paths: HashSet<PathBuf> = new_config
            .public
            .sync_paths
            .iter()
            .map(|p| PathBuf::from(p.to_string_lossy().trim().trim_matches('"')))
            .collect();

        new_config.public.sync_paths = sanitized_paths;

        if new_config
            .private
            .auth_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .is_none()
        {
            new_config.private.auth_key = None;
        }

        Self::save_update(&new_config).context("Failed to save configuration to file")?;

        {
            let mut w = APP_CONFIG.get().unwrap().write().unwrap();
            if new_config.private.auth_key.is_none() {
                FALLBACK_SECRET_KEY.get_or_init(generate_secret_key);
            }
            *w = new_config.clone();
        }

        reload_watcher();
        crate::public::db::write_behind::WRITE_BEHIND.config_updated();
        info!("Configuration updated successfully");
        Ok(())
    }

    fn save_update(config: &AppConfig) -> anyhow::Result<()> {
        let config_path = get_config_path();
        let config_path_display = config_path.display();

        let mut file = File::create(&config_path).context(format!(
            "Failed to create config file {config_path_display}"
        ))?;

        let pretty_json = serde_json::to_string_pretty(config)
            .context("Failed to serialize configuration to JSON")?;

        file.write_all(pretty_json.as_bytes()).context(format!(
            "Failed to write configuration to {config_path_display}"
        ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, WriteBehindConfig};

    #[test]
    fn legacy_config_uses_write_behind_defaults() {
        let mut value = serde_json::to_value(AppConfig::default()).unwrap();
        value
            .get_mut("public")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap()
            .remove("writeBehind");
        let decoded: AppConfig = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.public.write_behind, WriteBehindConfig::default());
    }

    #[test]
    fn write_behind_json_uses_public_mib_spelling() {
        let value = serde_json::to_value(WriteBehindConfig::default()).unwrap();
        assert_eq!(value["softLimitMiB"], 16);
        assert_eq!(value["hardLimitMiB"], 32);
        assert!(value.get("softLimitMib").is_none());

        let legacy: WriteBehindConfig = serde_json::from_value(serde_json::json!({
            "flushIntervalMs": 1000,
            "softLimitMib": 8,
            "hardLimitMib": 24
        }))
        .unwrap();
        assert_eq!(legacy.soft_limit_mib, 8);
        assert_eq!(legacy.hard_limit_mib, 24);
    }

    #[test]
    fn write_behind_validation_enforces_ranges_and_ordering() {
        assert!(WriteBehindConfig::default().validate().is_ok());
        assert!(
            WriteBehindConfig {
                flush_interval_ms: 99,
                ..WriteBehindConfig::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            WriteBehindConfig {
                soft_limit_mib: 32,
                hard_limit_mib: 32,
                ..WriteBehindConfig::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            WriteBehindConfig {
                hard_limit_mib: 257,
                ..WriteBehindConfig::default()
            }
            .validate()
            .is_err()
        );
    }
}
