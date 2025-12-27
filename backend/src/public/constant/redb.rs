use redb::TableDefinition;

use crate::public::structure::abstract_data::AbstractData;

pub const DATA_TABLE: TableDefinition<&str, AbstractData> = TableDefinition::new("database");
