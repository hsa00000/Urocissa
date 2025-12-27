use anyhow::Error;

use reqwest::blocking::Client;
use serde_json::json;

use crate::public::structure::config::APP_CONFIG;

pub fn handle_error(error: Error) -> Error {
    error!("{:?}", error);
    if let Some(url) = &APP_CONFIG.get().unwrap().read().unwrap().discord_hook_url {
        send_discord_webhook(url, &error);
    }
    error
}
fn send_discord_webhook(webhook_url: &str, error: &Error) -> () {
    let client = Client::new();
    let debug_string = format!("```rust\n{:?}\n```", error);
    let params = json!({ "content": debug_string });
    if let Err(e) = client.post(webhook_url).json(&params).send() {
        error!("Failed to send discord webhook: {}", e);
    }
}
