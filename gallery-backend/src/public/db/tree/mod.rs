pub mod read_tags;

use std::sync::{LazyLock, atomic::AtomicU64};
use crate::public::db::sqlite::SQLITE;
use std::sync::atomic::Ordering;

pub struct Tree;

pub static TREE: LazyLock<Tree> = LazyLock::new(|| Tree);


pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

pub fn initialize_from_db() {
    match SQLITE.get_latest_snapshot_timestamp() {
        Ok(Some(timestamp)) => {
            VERSION_COUNT_TIMESTAMP.store(timestamp as u64, Ordering::SeqCst);
            log::info!("Initialized VERSION_COUNT_TIMESTAMP to {}", timestamp);
        }
        Ok(None) => {
            log::info!("No snapshot found, VERSION_COUNT_TIMESTAMP starts at 0");
        }
        Err(e) => {
            log::error!("Failed to get latest snapshot timestamp: {}", e);
        }
    }
}
