use redb::TableDefinition;
use std::sync::LazyLock;
pub mod expired_check;
pub mod new;
pub static EXPIRE_TABLE_DEFINITION: TableDefinition<u64, Option<u64>> =
    TableDefinition::new("expire_table"); // timestamp -> expired time; none means never expired

#[derive(Debug)]
pub struct Expire {
    pub in_disk: &'static redb::Database,
}

pub static EXPIRE: LazyLock<Expire> = LazyLock::new(|| Expire::new());
