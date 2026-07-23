// src/public/structure/config.rs

use anyhow::Context;
use base64::{Engine as _, engine::general_purpose};
use log::{info, warn};
use rand::{TryRng, rngs::SysRng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use uuid::Uuid;

use crate::process::artifact_publisher::ArtifactPublisher;
use crate::public::constant::storage::get_config_path;
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::saved_search::{SavedSearch, normalize_and_validate_saved_searches};

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
    #[serde(default)]
    pub saved_searches: Vec<SavedSearch>,
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
                saved_searches: Vec::new(),
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

        if let Err(error) =
            normalize_and_validate_saved_searches(&mut config.private.saved_searches)
        {
            warn!("Invalid saved searches in configuration: {error}; using an empty list");
            config.private.saved_searches.clear();
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
        new_config.normalize_for_storage()?;
        Self::commit(new_config)?;

        reload_watcher();
        crate::public::db::write_behind::WRITE_BEHIND.config_updated();
        info!("Configuration updated successfully");
        Ok(())
    }

    /// Apply a partial configuration mutation while holding the configuration write lock from
    /// read through durable publication. This prevents concurrent settings requests from
    /// replacing one another with stale clones.
    pub fn mutate<R>(
        mutate: impl FnOnce(&mut AppConfig) -> Result<R, AppError>,
    ) -> Result<R, AppError> {
        let config_lock = APP_CONFIG.get().ok_or_else(|| {
            AppError::new(ErrorKind::Internal, "Configuration is not initialized")
        })?;
        let mut current = config_lock
            .write()
            .map_err(|_| AppError::new(ErrorKind::Internal, "Configuration lock is poisoned"))?;
        let result = Self::mutate_current(&mut current, mutate, Self::save_update)?;

        if current.private.auth_key.is_none() {
            FALLBACK_SECRET_KEY.get_or_init(generate_secret_key);
        }
        Ok(result)
    }

    fn mutate_current<R>(
        current: &mut AppConfig,
        mutate: impl FnOnce(&mut AppConfig) -> Result<R, AppError>,
        save: impl FnOnce(&AppConfig) -> anyhow::Result<()>,
    ) -> Result<R, AppError> {
        let mut next = current.clone();
        let result = mutate(&mut next)?;

        next.normalize_for_storage()
            .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))?;
        save(&next).map_err(|error| {
            AppError::from_err(ErrorKind::Internal, error)
                .context("Failed to save configuration to file")
        })?;

        *current = next;
        Ok(result)
    }

    fn normalize_for_storage(&mut self) -> anyhow::Result<()> {
        self.public.write_behind.validate()?;
        normalize_and_validate_saved_searches(&mut self.private.saved_searches)?;

        // Sanitize paths: only remove quotes and spaces, do not resolve paths.
        self.public.sync_paths = self
            .public
            .sync_paths
            .iter()
            .map(|path| PathBuf::from(path.to_string_lossy().trim().trim_matches('"')))
            .collect();

        if self
            .private
            .auth_key
            .as_ref()
            .filter(|key| !key.is_empty())
            .is_none()
        {
            self.private.auth_key = None;
        }

        Ok(())
    }

    fn commit(new_config: AppConfig) -> anyhow::Result<()> {
        let config_lock = APP_CONFIG
            .get()
            .context("Configuration is not initialized")?;
        let mut current = config_lock
            .write()
            .map_err(|_| anyhow::anyhow!("Configuration lock is poisoned"))?;
        Self::save_update(&new_config).context("Failed to save configuration to file")?;

        if new_config.private.auth_key.is_none() {
            FALLBACK_SECRET_KEY.get_or_init(generate_secret_key);
        }
        *current = new_config;
        Ok(())
    }

    fn save_update(config: &AppConfig) -> anyhow::Result<()> {
        let config_path = get_config_path();
        Self::save_update_at(config, &config_path)
    }

    fn save_update_at(config: &AppConfig, config_path: &Path) -> anyhow::Result<()> {
        let config_path_display = config_path.display();
        let parent = config_path.parent().context(format!(
            "Configuration path has no parent: {config_path_display}"
        ))?;
        fs::create_dir_all(parent).context(format!(
            "Failed to create configuration directory {}",
            parent.display()
        ))?;

        let pretty_json = serde_json::to_string_pretty(config)
            .context("Failed to serialize configuration to JSON")?;
        let mut publisher = ArtifactPublisher::new(format!("config-{}", Uuid::new_v4()));
        let staged_path = publisher.stage_path(config_path)?;
        publisher.replace(staged_path.clone(), config_path.to_path_buf());

        let mut file = File::create(&staged_path).context(format!(
            "Failed to create staged configuration file {}",
            staged_path.display()
        ))?;
        file.write_all(pretty_json.as_bytes()).context(format!(
            "Failed to write staged configuration {}",
            staged_path.display()
        ))?;
        file.sync_all().context(format!(
            "Failed to sync staged configuration {}",
            staged_path.display()
        ))?;
        drop(file);

        publisher.publish(|| Ok::<(), anyhow::Error>(()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier, RwLock};

    use tempfile::tempdir;

    use crate::public::error::ErrorKind;
    use crate::public::structure::saved_search::{SavedSearch, SavedSearchContext};

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
    fn legacy_config_uses_empty_saved_searches() {
        let mut value = serde_json::to_value(AppConfig::default()).unwrap();
        value
            .get_mut("private")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap()
            .remove("savedSearches");

        let decoded: AppConfig = serde_json::from_value(value).unwrap();
        assert!(decoded.private.saved_searches.is_empty());
    }

    #[test]
    fn saved_searches_stay_inside_private_config() {
        let value = serde_json::to_value(AppConfig::default()).unwrap();
        assert_eq!(value["private"]["savedSearches"], serde_json::json!([]));
        assert!(value["public"].get("savedSearches").is_none());
    }

    #[test]
    fn saved_searches_round_trip_with_camel_case_json() {
        let mut config = AppConfig::default();
        config.private.saved_searches.push(SavedSearch::new(
            "Family".to_owned(),
            SavedSearchContext::Favorite,
            "tag:family".to_owned(),
        ));

        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["private"]["savedSearches"][0]["name"], "Family");
        assert_eq!(value["private"]["savedSearches"][0]["context"], "favorite");
        assert!(value["private"].get("saved_searches").is_none());

        let decoded: AppConfig = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, config);
    }

    #[test]
    fn failed_transaction_keeps_memory_and_disk_unchanged() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.json");
        let original = AppConfig::default();
        std::fs::write(&path, serde_json::to_vec_pretty(&original).unwrap()).unwrap();
        let mut memory = original.clone();

        let result = AppConfig::mutate_current(
            &mut memory,
            |config| {
                config.public.port = 9_999;
                Ok(())
            },
            |_| anyhow::bail!("simulated write failure"),
        );

        assert_eq!(result.unwrap_err().kind, ErrorKind::Internal);
        assert_eq!(memory, original);
        let disk: AppConfig = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(disk, original);
    }

    #[test]
    fn concurrent_transactions_preserve_unrelated_config_changes() {
        let config = Arc::new(RwLock::new(AppConfig::default()));
        let start = Arc::new(Barrier::new(3));

        let public_config = Arc::clone(&config);
        let public_start = Arc::clone(&start);
        let public_change = std::thread::spawn(move || {
            public_start.wait();
            let mut current = public_config.write().unwrap();
            AppConfig::mutate_current(
                &mut current,
                |next| {
                    next.public.port = 9_999;
                    Ok(())
                },
                |_| Ok(()),
            )
            .unwrap();
        });

        let saved_search_config = Arc::clone(&config);
        let saved_search_start = Arc::clone(&start);
        let saved_search_change = std::thread::spawn(move || {
            saved_search_start.wait();
            let mut current = saved_search_config.write().unwrap();
            AppConfig::mutate_current(
                &mut current,
                |next| {
                    next.private.saved_searches.push(SavedSearch::new(
                        "Family".to_owned(),
                        SavedSearchContext::Home,
                        "tag:family".to_owned(),
                    ));
                    Ok(())
                },
                |_| Ok(()),
            )
            .unwrap();
        });

        start.wait();
        public_change.join().unwrap();
        saved_search_change.join().unwrap();

        let config = config.read().unwrap();
        assert_eq!(config.public.port, 9_999);
        assert_eq!(config.private.saved_searches.len(), 1);
    }

    #[test]
    fn atomic_save_replaces_existing_config() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.json");
        std::fs::write(&path, b"old").unwrap();

        let config = AppConfig::default();
        AppConfig::save_update_at(&config, &path).unwrap();

        let decoded: AppConfig = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(decoded, config);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
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
