use crate::public::db::tree_snapshot::{TREE_SNAPSHOT, read_tree_snapshot::MyCow};
use anyhow::Context;
use anyhow::Result;

pub fn open_tree_snapshot_table(timestamp: u128) -> Result<MyCow> {
    TREE_SNAPSHOT
        .read_tree_snapshot(&timestamp)
        .context(format!(
            "Failed to read tree snapshot for timestamp {}",
            timestamp
        ))
}
