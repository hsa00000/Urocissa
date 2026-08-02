use directories::ProjectDirs;
use log::{error, info};
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

pub static DATA_PATH: OnceLock<PathBuf> = OnceLock::new();

const CONFIG_FILE_NAME: &str = "config.json";

/// Returns the path to the config file based on the current mode (portable or installed).
/// In portable mode, returns "./config.json".
/// In installed mode, returns `<data_path>/config.json`.
pub fn get_config_path() -> PathBuf {
    get_data_path().join(CONFIG_FILE_NAME)
}

pub fn get_data_path() -> &'static PathBuf {
    DATA_PATH.get_or_init(|| {
        #[cfg(feature = "performance-test")]
        if let Some(perf_root) = std::env::var_os("UROCISSA_PERF_ROOT") {
            return PathBuf::from(perf_root);
        }

        // 1. Check for portable marker or existing directories
        // The user said: "first check portable db and object"

        let portable_db = Path::new("db");
        let portable_object = Path::new("object");

        // If "db" or "object" folder exists in current directory, assume portable mode
        if portable_db.exists() || portable_object.exists() {
            info!("Portable mode detected (found ./db or ./object)");
            return PathBuf::from(".");
        }

        // 2. Fallback to installed mode (AppData/XDG_DATA_HOME)
        if let Some(proj_dirs) = ProjectDirs::from("com", "urocissa", "urocissa") {
            let data_dir = proj_dirs.data_dir().to_path_buf();

            // Create the directory if it doesn't exist
            if !data_dir.exists()
                && let Err(e) = std::fs::create_dir_all(&data_dir)
            {
                error!(
                    "Failed to create data directory {}: {e}",
                    data_dir.display()
                );
                // Fallback to local if we can't write to AppData
                return PathBuf::from(".");
            }

            info!(
                "Installed mode detected. Using data directory: {}",
                data_dir.display()
            );
            return data_dir;
        }

        // 3. Fallback to current directory if ProjectDirs fails
        info!("Could not determine system data directory. Defaulting to portable mode.");
        PathBuf::from(".")
    })
}
