use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
// 新增引入
use rand::{TryRngCore, rngs::OsRng};
use std::sync::{LazyLock, OnceLock, RwLock};

// Refactor: Renamed AppSettings to AppConfig
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// 管理員密碼 (明文)
    pub password: String,
    /// 需要監聽/同步的資料夾路徑
    pub sync_paths: HashSet<PathBuf>,
    /// 驗證金鑰 (JWT Secret)
    pub auth_key: Option<String>,
    /// Discord Webhook URL
    pub discord_hook_url: Option<String>,
    /// 唯讀模式 (不允許上傳/刪除)
    pub read_only_mode: bool,
    /// 禁用圖片處理 (僅顯示檔案)
    pub disable_img: bool,
    /// 上傳檔案大小限制 (MB)
    pub upload_limit_mb: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            password: "admin".to_string(),
            sync_paths: HashSet::new(),
            auth_key: None,
            discord_hook_url: None,
            read_only_mode: false,
            disable_img: false,
            upload_limit_mb: 2048, // 預設 2GB
        }
    }
}

pub static APP_CONFIG: OnceLock<RwLock<AppConfig>> = OnceLock::new();

static FALLBACK_SECRET_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut secret = vec![0u8; 32];
    OsRng
        .try_fill_bytes(&mut secret)
        .expect("Failed to generate random secret key");
    secret
});

impl AppConfig {
    /// 獲取 JWT Secret Key (實例方法)
    pub fn get_jwt_secret_key(&self) -> Vec<u8> {
        match self.auth_key.as_ref() {
            Some(auth_key) => auth_key.as_bytes().to_vec(),
            None => FALLBACK_SECRET_KEY.clone(),
        }
    }
}
