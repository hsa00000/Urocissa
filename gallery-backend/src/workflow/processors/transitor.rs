//! Data transformation utilities
//!
//! Includes:
//! - Index to hash conversion
//! - Abstract data metadata clearing
//! - Response data processing
//! - Timestamp utilities
//! - Permission resolution

use crate::public::{
    db::tree_snapshot::read_tree_snapshot::MyCow, structure::abstract_data::AbstractData,
};
use crate::table::relations::album_share::ResolvedShare;
use anyhow::Result;
use arrayvec::ArrayString;
use std::time::{SystemTime, UNIX_EPOCH};

// ────────────────────────────────────────────────────────────────
// Index/Hash Conversion
// ────────────────────────────────────────────────────────────────

/// Convert an index to its corresponding hash
pub fn index_to_hash(tree_snapshot: &MyCow, index: usize) -> Result<ArrayString<64>> {
    if index >= tree_snapshot.len() {
        return Err(anyhow::anyhow!("Index out of bounds: {}", index));
    }
    let hash = tree_snapshot.get_hash(index)?;
    Ok(hash)
}

// ────────────────────────────────────────────────────────────────
// Metadata Clearing
// ────────────────────────────────────────────────────────────────

/// Process abstract data for API response
pub fn process_abstract_data_for_response(
    abstract_data: AbstractData,
    _show_metadata: bool,
) -> AbstractData {
    match abstract_data {
        AbstractData::Image(_) | AbstractData::Video(_) => {
            // 媒體的 metadata 清除，暫時保持
        }
        AbstractData::Album(_) => {
            // Album 的 tag 現在從關聯表讀取，不需要清除
        }
    }
    abstract_data
}

// ────────────────────────────────────────────────────────────────
// Timestamp Utilities
// ────────────────────────────────────────────────────────────────

/// Get the current timestamp in milliseconds since UNIX epoch
pub fn get_current_timestamp_u64() -> u64 {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    timestamp as u64
}

// ────────────────────────────────────────────────────────────────
// Permission Resolution
// ────────────────────────────────────────────────────────────────

/// Resolve show_download and show_metadata flags from share permissions
pub fn resolve_show_download_and_metadata(
    resolved_share_opt: Option<ResolvedShare>,
) -> (bool, bool) {
    resolved_share_opt.map_or((true, true), |resolved_share| {
        (
            resolved_share.share.show_download,
            resolved_share.share.show_metadata,
        )
    })
}
