use super::delete::generate_delete_routes;
use super::fairing::cache_control_fairing::cache_control_fairing;
use super::fairing::generate_fairing_routes;
use super::get::generate_get_routes;
use super::post::generate_post_routes;
use super::put::generate_put_routes;
use crate::public::constant::storage::get_config_path;
use crate::public::structure::config::AppConfig;

use figment::{
    Figment,
    providers::{Format, Json},
};
use rocket::data::{ByteUnit, Limits};
#[cfg(not(feature = "embed-frontend"))]
use rocket::fs::FileServer;
use rocket::info;
use std::path::PathBuf;

// Use alias to avoid conflict with Rocket's ByteUnit
use ubyte::ByteUnit as HumanBytes;

#[cfg(test)]
fn create_dummy_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.public.address = "127.0.0.1".to_string();
    config.public.port = 8000;
    config
}

/// Handle routes for embedded frontend assets
#[cfg(feature = "embed-frontend")]
#[get("/assets/<file..>")]
#[allow(clippy::needless_pass_by_value)]
fn assets(file: PathBuf) -> Option<(rocket::http::ContentType, std::borrow::Cow<'static, [u8]>)> {
    use crate::public::embedded::FrontendAssets;

    let filename = format!("assets/{}", file.display());
    let asset = FrontendAssets::get(&filename)?;

    // Simplify MIME type handling logic
    let mime = mime_guess::from_path(&filename).first_or_octet_stream();
    let content_type = rocket::http::ContentType::parse_flexible(mime.as_ref())
        .unwrap_or(rocket::http::ContentType::Binary);

    Some((content_type, asset.data))
}

/// Load configuration from the file system
pub fn load_config() -> AppConfig {
    let config_path = get_config_path();
    let figment = Figment::new().merge(Json::file(&config_path));

    figment
        .extract()
        .expect("config.json format error or type mismatch")
}

/// Helper to parse limits from config
fn extract_limit(app_config: &AppConfig, key: &str, default_val: &str) -> ByteUnit {
    let raw_val = app_config
        .public
        .limits
        .get(key)
        .map_or(default_val, std::string::String::as_str);

    let bytes = raw_val
        .parse::<HumanBytes>()
        .unwrap_or_else(|_| panic!("Invalid limit format for key '{key}': {raw_val}"))
        .as_u64();

    ByteUnit::from(bytes)
}

#[cfg(test)]
mod test_extract_limit {
    use super::*;

    #[test]
    fn test_extract_limit_defaults() {
        let config = create_dummy_config();

        let limit = extract_limit(&config, "non_existent", "1KiB");
        assert_eq!(limit, ByteUnit::KiB);

        let limit = extract_limit(&config, "non_existent", "1MiB");
        assert_eq!(limit, ByteUnit::MiB);
    }

    #[test]
    fn test_extract_limit_custom() {
        let mut config = create_dummy_config();
        config
            .public
            .limits
            .insert("custom".to_string(), "512KiB".to_string());

        let limit = extract_limit(&config, "custom", "1MiB");
        assert_eq!(limit, ByteUnit::from(512 * 1024));
    }

    #[test]
    #[should_panic(expected = "Invalid limit format")]
    fn test_extract_limit_invalid() {
        let mut config = create_dummy_config();
        config
            .public
            .limits
            .insert("bad".to_string(), "invalid_unit".to_string());

        extract_limit(&config, "bad", "1MiB");
    }
}

/// Build and configure Rocket instance
pub fn build_rocket() -> rocket::Rocket<rocket::Build> {
    let app_config = load_config();
    build_rocket_with_config(app_config)
}

/// Build Rocket instance with injected configuration
pub fn build_rocket_with_config(app_config: AppConfig) -> rocket::Rocket<rocket::Build> {
    let limits = Limits::default()
        .limit("form", extract_limit(&app_config, "data-form", "10GiB"))
        .limit(
            "data-form",
            extract_limit(&app_config, "data-form", "10GiB"),
        )
        .limit("file", extract_limit(&app_config, "file", "10GiB"))
        .limit("json", extract_limit(&app_config, "json", "10MiB"));

    let rocket_config = rocket::Config::figment()
        .merge(("address", &app_config.public.address))
        .merge(("port", app_config.public.port))
        .merge(("limits", limits));

    let base_app = rocket::custom(rocket_config)
        .manage(app_config)
        .attach(cache_control_fairing());

    let app = mount_frontend(base_app);

    app.mount("/", generate_get_routes())
        .mount("/", generate_post_routes())
        .mount("/", generate_put_routes())
        .mount("/", generate_delete_routes())
        .mount("/", generate_fairing_routes())
        .mount("/", super::performance::generate_performance_routes())
}

#[cfg(test)]
mod test_build_rocket_with_config {
    use super::*;

    #[test]
    fn test_build_rocket_config() {
        let mut config = create_dummy_config();
        config.public.port = 9999;
        config.public.address = "0.0.0.0".to_string();
        config
            .public
            .limits
            .insert("json".to_string(), "20MiB".to_string());

        let rocket = build_rocket_with_config(config);
        let figment = rocket.figment();

        let port: u16 = figment.extract_inner("port").expect("port not found");
        let address: String = figment.extract_inner("address").expect("address not found");

        assert_eq!(port, 9999);
        assert_eq!(address, "0.0.0.0");

        // Limits in figment are stored under "limits"
        // We can check if the value propagated to Figment correctly
        // rocket::Config::figment() merges limits
        let limits: rocket::data::Limits =
            figment.extract_inner("limits").expect("limits not found");
        assert_eq!(limits.get("json"), Some(ByteUnit::MiB * 20));
    }
}

fn mount_frontend(app: rocket::Rocket<rocket::Build>) -> rocket::Rocket<rocket::Build> {
    #[cfg(feature = "embed-frontend")]
    {
        use rocket::routes; // Import routes only here
        info!("Serving assets from embedded binary");
        app.mount("/", routes![assets])
    }

    #[cfg(not(feature = "embed-frontend"))]
    {
        // Development mode: Assume frontend dist folder is in sibling directory
        let asset_path = PathBuf::from("../gallery-frontend/dist/assets");
        info!("Serving assets from: {:?}", asset_path);
        app.mount("/assets", FileServer::from(asset_path))
    }
}
