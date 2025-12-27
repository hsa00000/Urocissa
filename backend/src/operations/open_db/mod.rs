use crate::public::{
    constant::redb::DATA_TABLE,
    db::{
        tree::TREE,
        tree_snapshot::{TREE_SNAPSHOT, read_tree_snapshot::MyCow},
    },
    structure::abstract_data::AbstractData,
};
use anyhow::Context;
use anyhow::Result;
use redb::{ReadOnlyTable, ReadableDatabase};

pub fn open_data_table() -> ReadOnlyTable<&'static str, AbstractData> {
    let read_txn = TREE.in_disk.begin_read().unwrap();
    read_txn.open_table(DATA_TABLE).unwrap()
}

pub fn open_tree_snapshot_table(timestamp: u128) -> Result<MyCow> {
    TREE_SNAPSHOT
        .read_tree_snapshot(&timestamp)
        .context(format!(
            "Failed to read tree snapshot for timestamp {}",
            timestamp
        ))
}
