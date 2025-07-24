use redb::TableDefinition;

use crate::public::structure::{album::Album, database_struct::database::definition::Database};

pub const DATA_TABLE: TableDefinition<&str, Database> = TableDefinition::new("database");

pub const ALBUM_TABLE: TableDefinition<&str, Album> = TableDefinition::new("album");
