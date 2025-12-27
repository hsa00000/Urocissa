use crate::public::structure::album::ResolvedShare;

pub mod hash;
pub mod indexation;
pub mod initialization;
pub mod open_db;
pub mod open_file;
pub mod transitor;
pub mod utils;

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
