use std::{fs, panic::Location, path::PathBuf};

use arrayvec::ArrayString;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::public::config::PRIVATE_CONFIG;

use super::database_struct::database::definition::Database;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct JsonData(pub &'static str, pub &'static std::path::Path);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorData<'a> {
    pub error: String,
    pub description: String,
    pub hash: Option<ArrayString<64>>,
    pub file: Option<PathBuf>,
    pub time: u128,
    pub location: &'a Location<'a>,
    pub remove_file: Option<Database>,
}

impl<'a> ErrorData<'a> {
    pub fn new(
        error: String,
        description: String,
        hash: Option<ArrayString<64>>,
        file: Option<PathBuf>,
        location: &'a Location,
        remove_file: Option<Database>,
    ) -> Self {
        ErrorData {
            error,
            description,
            time: Utc::now().timestamp_millis() as u128,
            location,
            hash,
            file,
            remove_file,
        }
    }
    pub fn to_formatted_string(&self) -> String {
        format!(
            "**Error**: {}\n**Description**: {}\n**File**: {}\n**Location**: {}\n**Time**: {}",
            self.error,
            self.description,
            self.file
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("None"))
                .display(),
            self.location,
            self.time,
        )
    }
}

pub fn handle_error(error_data: ErrorData) -> () {
    error!("{:#?}", error_data);
    if let Some(database) = &error_data.remove_file {
        let remove_file_list = vec![
            database.imported_path(),
            database.compressed_path(),
            database.thumbnail_path().into(),
        ];
        for file_path in remove_file_list {
            if file_path.exists() {
                match fs::remove_file(&file_path) {
                    Ok(_) => {
                        info!("Successfully removed file {}", file_path.display());
                    }
                    Err(e) => {
                        error!("Failed to remove file {}: {}", file_path.display(), e);
                    }
                }
            } else {
                info!("File {} does not exist. Skip delete", file_path.display());
            }
        }
    }
    if let Some(url) = &PRIVATE_CONFIG.discord_hook_url {
        send_discord_webhook(url, error_data);
    }
}
fn send_discord_webhook(webhook_url: &str, error_data: ErrorData) -> () {
    let client = reqwest::blocking::Client::new();
    let debug_string = error_data.to_formatted_string();
    let params = json!({ "content": debug_string });
    let result = client.post(webhook_url).json(&params).send();
    if let Err(e) = result {
        error!("Error sending webhook: {:#?}", e);
    }
}
