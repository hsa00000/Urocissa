use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
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

impl Default for AppSettings {
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
