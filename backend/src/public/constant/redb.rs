use redb::TableDefinition;

use crate::public::structure::abstract_data::AbstractData;

// 主要資料表
pub const DATA_TABLE: TableDefinition<&str, AbstractData> = TableDefinition::new("database");

// 儲存全域設定 (Config)，Key 固定為 "app_config"
// Refactor: Table name changed from "settings" to "config"
pub const CONFIG_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("config");
