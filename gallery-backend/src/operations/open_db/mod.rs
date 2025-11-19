use crate::public::db::tree_snapshot::{TREE_SNAPSHOT, read_tree_snapshot::SnapshotReader};
use anyhow::Context;
use anyhow::Result;

pub fn open_tree_snapshot_table(timestamp: u128) -> Result<SnapshotReader> {
    TREE_SNAPSHOT
        .read_tree_snapshot(&timestamp)
        .context(format!(
            "Failed to read tree snapshot for timestamp {}",
            timestamp
        ))
}

