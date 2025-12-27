use redb::Database;
use std::sync::LazyLock;

pub static SETTINGS_DB: LazyLock<Database> = LazyLock::new(|| {
    // 建立或開啟獨立的設定資料庫
    Database::create("./db/setting.redb").expect("Failed to create setting.redb")
});
