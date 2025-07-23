use crate::public::{
    constant::redb::DATA_TABLE,
    db::{
        tree::TREE,
        tree_snapshot::{TREE_SNAPSHOT, read_tree_snapshot::MyCow},
    },
    structure::{album::Album, database_struct::database::definition::Database},
};
use anyhow::Context;
use anyhow::Result;
use redb::ReadOnlyTable;

pub fn open_data_table() -> Result<ReadOnlyTable<&'static str, Database>> {
    let read_txn = TREE
        .in_disk
        .begin_read()
        .context("Begin read transaction failed")?;

    let data_table = read_txn
        .open_table(DATA_TABLE)
        .context("Open DATA_TABLE failed")?;

    Ok(data_table)
}

pub fn open_album_table() -> ReadOnlyTable<&'static str, Album> {
    let read_txn = TREE.in_disk.begin_read().unwrap();
    let album_table = read_txn
        .open_table(crate::public::constant::redb::ALBUM_TABLE)
        .unwrap();
    album_table
}

pub fn open_data_and_album_tables() -> (
    ReadOnlyTable<&'static str, Database>,
    ReadOnlyTable<&'static str, Album>,
) {
    let read_txn = TREE.in_disk.begin_read().unwrap();
    let data_table = read_txn.open_table(DATA_TABLE).unwrap();
    let album_table = read_txn
        .open_table(crate::public::constant::redb::ALBUM_TABLE)
        .unwrap();
    (data_table, album_table)
}

pub fn open_tree_snapshot_table(timestamp: u128) -> Result<MyCow> {
    TREE_SNAPSHOT
        .read_tree_snapshot(&timestamp)
        .context(format!(
            "Failed to read tree snapshot for timestamp {}",
            timestamp
        ))
}
