use dotenv::dotenv;
use log::info;
use redb::ReadableDatabase;
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::{OnceLock, RwLock},
};

use crate::public::constant::redb::SETTINGS_TABLE;
// 引入新的 SETTINGS_DB
use crate::public::db::settings::SETTINGS_DB;
// 保留 TREE 以便進行遷移 (Migration)
use crate::public::db::tree::TREE;
use crate::public::structure::settings::AppSettings;

// 全域設定儲存區 (New dynamic config system)
pub static CONFIG: OnceLock<RwLock<AppSettings>> = OnceLock::new();

/// 獲取當前設定的快照 (Clone)
pub fn get_config() -> AppSettings {
    CONFIG
        .get()
        .expect("Config not initialized")
        .read()
        .unwrap()
        .clone()
}

/// 初始化設定系統 (請在 main 最開始呼叫)
pub fn init_config() {
    let mut settings = AppSettings::default();
    let mut db_has_data = false;

    // 1. 嘗試從 新的 DB (setting.redb) 讀取
    // 使用 SETTINGS_DB
    let read_txn = SETTINGS_DB.begin_read().expect("Failed to begin read txn");
    if let Ok(table) = read_txn.open_table(SETTINGS_TABLE) {
        if let Ok(Some(value)) = table.get("app_settings") {
            if let Ok(saved_settings) = serde_json::from_slice::<AppSettings>(value.value()) {
                info!("Loaded settings from database (setting.redb).");
                settings = saved_settings;
                db_has_data = true;
            }
        }
    }

    // 1.5 如果新 DB 沒資料，嘗試從 舊 DB (index.redb / TREE) 遷移
    if !db_has_data {
        let read_txn_old = TREE
            .in_disk
            .begin_read()
            .expect("Failed to begin read txn for old db");
        if let Ok(table) = read_txn_old.open_table(SETTINGS_TABLE) {
            if let Ok(Some(value)) = table.get("app_settings") {
                if let Ok(saved_settings) = serde_json::from_slice::<AppSettings>(value.value()) {
                    info!(
                        "Migrating settings from old database (index.redb) to new (setting.redb)..."
                    );
                    settings = saved_settings;
                    // 立即寫入新 DB
                    save_to_db(&settings).expect("Failed to save migrated settings to new DB");
                    db_has_data = true;
                }
            }
        }
    }

    // 2. 如果 DB 沒資料 (第一次運行或遷移)，嘗試讀取舊設定檔 (legacy files)
    if !db_has_data {
        info!("No settings in DB, migrating from legacy files...");
        settings = migrate_from_legacy_files();
        // 立即寫入 DB
        save_to_db(&settings).expect("Failed to save migrated settings to DB");
    }

    // 3. 設定到全域記憶體
    CONFIG
        .set(RwLock::new(settings))
        .expect("Config already initialized");
    println!("CONFIG is {:?}", CONFIG)
}

/// 更新設定並觸發副作用 (如重啟 Watcher)
pub fn update_config(new_settings: AppSettings) -> anyhow::Result<()> {
    use crate::tasks::batcher::start_watcher::reload_watcher;

    // 1. 寫入 DB
    save_to_db(&new_settings)?;

    // 2. 更新記憶體
    {
        let mut w = CONFIG.get().unwrap().write().unwrap();
        *w = new_settings.clone();
    }

    // 3. 觸發 Watcher 重啟 (因為 sync_paths 可能變了)
    reload_watcher();

    Ok(())
}

fn save_to_db(settings: &AppSettings) -> anyhow::Result<()> {
    // 改用 SETTINGS_DB
    let write_txn = SETTINGS_DB.begin_write()?;
    {
        let mut table = write_txn.open_table(SETTINGS_TABLE)?;
        let json_bytes = serde_json::to_vec(settings)?;
        table.insert("app_settings", json_bytes.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 從舊的 .env 和 config.json 讀取資料 (遷移用)
fn migrate_from_legacy_files() -> AppSettings {
    let mut settings = AppSettings::default();

    // 讀取 config.json
    if let Ok(file) = File::open("config.json") {
        #[derive(serde::Deserialize)]
        struct OldPublic {
            read_only_mode: bool,
            disable_img: bool,
        }
        if let Ok(old) = serde_json::from_reader::<_, OldPublic>(file) {
            settings.read_only_mode = old.read_only_mode;
            settings.disable_img = old.disable_img;
        }
    }

    // 讀取 .env
    dotenv().ok();
    if let Ok(pwd) = std::env::var("PASSWORD") {
        settings.password = pwd;
    }
    if let Ok(key) = std::env::var("AUTH_KEY") {
        settings.auth_key = Some(key);
    }
    if let Ok(hook) = std::env::var("DISCORD_HOOK_URL") {
        if !hook.trim().is_empty() {
            settings.discord_hook_url = Some(hook);
        }
    }

    // 處理 SYNC_PATH
    if let Ok(sync_paths_str) = std::env::var("SYNC_PATH") {
        for path_str in sync_paths_str.split(',') {
            let path = PathBuf::from(path_str.trim());
            if !path_str.trim().is_empty() {
                settings.sync_paths.insert(path);
            }
        }
    }

    // 過濾掉 upload 路徑
    if let Ok(upload_path) = fs::canonicalize(PathBuf::from("./upload")) {
        settings.sync_paths.retain(|p| match fs::canonicalize(p) {
            Ok(c) => c != upload_path,
            Err(_) => p != &upload_path,
        });
    }

    settings
}
