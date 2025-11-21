use crate::public::{
    db::{
        tree::TREE,
        tree_snapshot::{TREE_SNAPSHOT, read_tree_snapshot::MyCow},
    },
    structure::{album::Album, database_struct::database::definition::Database},
};
use anyhow::Context;
use anyhow::Result;
use redb::ReadOnlyTable;

pub fn open_tree_snapshot_table(timestamp: u128) -> Result<MyCow> {
    TREE_SNAPSHOT
        .read_tree_snapshot(&timestamp)
        .context(format!(
            "Failed to read tree snapshot for timestamp {}",
            timestamp
        ))
}
