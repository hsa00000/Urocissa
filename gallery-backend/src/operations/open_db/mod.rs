use crate::public::db::{
    tree::TREE,
    tree_snapshot::{TREE_SNAPSHOT, read_tree_snapshot::MyCow},
};
use crate::storage::store::RecordReader;
use anyhow::Context;
use anyhow::Result;

pub fn open_data_table() -> RecordReader {
    TREE.store.reader().unwrap()
}

pub fn open_tree_snapshot_table(timestamp: i64) -> Result<MyCow> {
    TREE_SNAPSHOT.read_tree_snapshot(timestamp).context(format!(
        "Failed to read tree snapshot for timestamp {timestamp}"
    ))
}
