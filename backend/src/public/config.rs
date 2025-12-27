use dotenv::dotenv;
use log::info;
use redb::ReadableDatabase;
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::RwLock,
};

// Refactor: Update imports to use CONFIG_TABLE and CONFIG_DB
use crate::public::constant::redb::CONFIG_TABLE;
use crate::public::db::config::CONFIG_DB;
// 保留 TREE 以便進行遷移 (Migration from old index.redb)
use crate::public::db::tree::TREE;
// Refactor: Use AppConfig
use crate::public::structure::config::{APP_CONFIG, AppConfig};

// 移除 pub static CONFIG ...
// 移除 pub fn get_config() ...

/// 初始化設定系統
pub fn init_config() {
    let mut config = AppConfig::default();
    let mut db_has_data = false;

    // 1. 嘗試從 新的 DB (config.redb) 讀取
    let read_txn = CONFIG_DB.begin_read().expect("Failed to begin read txn");
    if let Ok(table) = read_txn.open_table(CONFIG_TABLE) {
        // Key changed to "app_config"
        if let Ok(Some(value)) = table.get("app_config") {
            if let Ok(saved_config) = serde_json::from_slice::<AppConfig>(value.value()) {
                info!("Loaded config from database (config.redb).");
                config = saved_config;
                db_has_data = true;
            }
        }
    }

    // 1.5 如果新 DB 沒資料，嘗試從 舊 DB (index.redb / TREE) 遷移
    // 這裡我們需要暫時引用舊的 TABLE 定義來讀取，或者直接硬編碼字串
    if !db_has_data {
        use redb::TableDefinition;
        // 舊的設定表定義
        const OLD_SETTINGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");

        let read_txn_old = TREE
            .in_disk
            .begin_read()
            .expect("Failed to begin read txn for old db");

        if let Ok(table) = read_txn_old.open_table(OLD_SETTINGS_TABLE) {
            if let Ok(Some(value)) = table.get("app_settings") {
                // 嘗試將舊的 AppSettings (結構可能相同) 反序列化為新的 AppConfig
                if let Ok(saved_settings) = serde_json::from_slice::<AppConfig>(value.value()) {
                    info!(
                        "Migrating settings from old database (index.redb) to new (config.redb)..."
                    );
                    config = saved_settings;
                    // 立即寫入新 DB
                    save_to_db(&config).expect("Failed to save migrated config to new DB");
                    db_has_data = true;
                }
            }
        }
    }

    // 2. 如果 DB 沒資料 (第一次運行或遷移)，嘗試讀取舊設定檔 (legacy files)
    if !db_has_data {
        info!("No config in DB, migrating from legacy files...");
        config = migrate_from_legacy_files();
        // 立即寫入 DB
        save_to_db(&config).expect("Failed to save migrated config to DB");
    }

    // 3. 設定到全域記憶體 APP_CONFIG
    APP_CONFIG
        .set(RwLock::new(config))
        .expect("Config already initialized");
    println!("APP_CONFIG is {:?}", APP_CONFIG)
}

/// 更新設定並觸發副作用
pub fn update_config(new_config: AppConfig) -> anyhow::Result<()> {
    use crate::tasks::batcher::start_watcher::reload_watcher;

    // 1. 寫入 DB
    save_to_db(&new_config)?;

    // 2. 更新記憶體
    {
        // 使用 APP_CONFIG
        let mut w = APP_CONFIG.get().unwrap().write().unwrap();
        *w = new_config.clone();
    }

    // 3. 觸發 Watcher 重啟
    reload_watcher();

    Ok(())
}

fn save_to_db(config: &AppConfig) -> anyhow::Result<()> {
    let write_txn = CONFIG_DB.begin_write()?;
    {
        let mut table = write_txn.open_table(CONFIG_TABLE)?;
        let json_bytes = serde_json::to_vec(config)?;
        // Key is now "app_config"
        table.insert("app_config", json_bytes.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 從舊的 .env 和 config.json 讀取資料 (遷移用)
fn migrate_from_legacy_files() -> AppConfig {
    let mut config = AppConfig::default();

    // 讀取 config.json
    if let Ok(file) = File::open("config.json") {
        #[derive(serde::Deserialize)]
        struct OldPublic {
            read_only_mode: bool,
            disable_img: bool,
        }
        if let Ok(old) = serde_json::from_reader::<_, OldPublic>(file) {
            config.read_only_mode = old.read_only_mode;
            config.disable_img = old.disable_img;
        }
    }

    // 讀取 .env
    dotenv().ok();
    if let Ok(pwd) = std::env::var("PASSWORD") {
        config.password = pwd;
    }
    if let Ok(key) = std::env::var("AUTH_KEY") {
        config.auth_key = Some(key);
    }
    if let Ok(hook) = std::env::var("DISCORD_HOOK_URL") {
        if !hook.trim().is_empty() {
            config.discord_hook_url = Some(hook);
        }
    }

    // 處理 SYNC_PATH
    if let Ok(sync_paths_str) = std::env::var("SYNC_PATH") {
        for path_str in sync_paths_str.split(',') {
            let path = PathBuf::from(path_str.trim());
            if !path_str.trim().is_empty() {
                config.sync_paths.insert(path);
            }
        }
    }

    // 過濾掉 upload 路徑
    if let Ok(upload_path) = fs::canonicalize(PathBuf::from("./upload")) {
        config.sync_paths.retain(|p| match fs::canonicalize(p) {
            Ok(c) => c != upload_path,
            Err(_) => p != &upload_path,
        });
    }

    config
}
