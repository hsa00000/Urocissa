use redb::TableDefinition;
use serde::{Deserialize, Serialize};

// Key: (Hash, ScanTime) -> Value: DatabaseAliasSchema
pub const DATABASE_ALIAS_TABLE: TableDefinition<(&str, i64), &[u8]> =
    TableDefinition::new("database_alias");

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DatabaseAliasSchema {
    pub hash: String,
    pub file: String,
    pub modified: i64,
    pub scan_time: i64,
}

pub struct DatabaseAliasTable;
// CRUD logic can be implemented here directly using txn.open_table(DATABASE_ALIAS_TABLE)
