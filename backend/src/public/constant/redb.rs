use redb::TableDefinition;

use crate::public::structure::abstract_data::AbstractData;

pub const DATA_TABLE: TableDefinition<&str, AbstractData> = TableDefinition::new("database");

// 儲存全域設定，Key 固定為 "app_settings"
pub const SETTINGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");
