use super::TreeSnapshot;
use crate::public::db::sqlite::SQLITE;
use anyhow::Result;
use arrayvec::ArrayString;

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: &u128) -> Result<SnapshotReader> {
        Ok(SnapshotReader {
            timestamp: *timestamp,
        })
    }
}

#[derive(Debug)]
pub struct SnapshotReader {
    timestamp: u128,
}

impl SnapshotReader {
    pub fn len(&self) -> usize {
        SQLITE.get_snapshot_len(self.timestamp).unwrap_or(0)
    }

    pub fn get_width_height(&self, index: usize) -> Result<(u32, u32)> {
        Ok(SQLITE.get_snapshot_width_height(self.timestamp, index)?)
    }

    pub fn get_hash(&self, index: usize) -> Result<ArrayString<64>> {
        let hash_str = SQLITE.get_snapshot_hash(self.timestamp, index)?;
        Ok(ArrayString::from(&hash_str).unwrap_or_default())
    }
}
