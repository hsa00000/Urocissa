use std::sync::atomic::Ordering;

use arrayvec::ArrayString;
use rand::RngExt;
use rand::distr::Alphanumeric;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::album::Share;
use crate::public::structure::object::next_mutation_timestamp;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateShare {
    pub album_id: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: i64,
}

#[post("/post/create_share", data = "<create_share>")]
pub async fn create_share(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    create_share: Json<CreateShare>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = create_share.into_inner();
    let link = rand::rng()
        .sample_iter(&Alphanumeric)
        .filter(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        .take(64)
        .map(char::from)
        .collect::<String>();
    let share_id = ArrayString::<64>::from(&link)
        .map_err(|_| AppError::new(ErrorKind::Internal, "failed to create share ID"))?;
    let share = Share {
        url: share_id,
        description: data.description,
        password: data.password,
        show_metadata: data.show_metadata,
        show_download: data.show_download,
        show_upload: data.show_upload,
        exp: data.exp,
    };
    let reservation = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        let mut preview = state
            .albums
            .get(&data.album_id)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorKind::NotFound, "album not found"))?;
        preview.metadata.share_list.insert(share_id, share.clone());
        DirtyOperation::AlbumReplace(preview).estimated_bytes() + 1_024
    };
    WRITE_BEHIND.reserve(reservation).await?;
    let mut state = TREE.state.write().map_err(|_| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::Internal, "tree state lock poisoned")
    })?;
    let universe = state.arena.capacity();
    let album_slot = state.find(data.album_id.as_str()).ok_or_else(|| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::NotFound, "album not found")
    })?;
    let changed_at = next_mutation_timestamp();
    let album = state.albums.get_mut(&data.album_id).ok_or_else(|| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::NotFound, "album not found")
    })?;
    album.metadata.share_list.insert(share_id, share);
    album.object.touch_update_at(changed_at);
    let album = album.clone();
    WRITE_BEHIND.enqueue_reserved(DirtyOperation::AlbumReplace(album), reservation);
    WRITE_BEHIND.enqueue_reserved(
        DirtyOperation::Touch {
            targets: crate::public::db::tree::state::TargetSet::from_slot_refs(
                [album_slot],
                universe,
            ),
            changed_at,
        },
        0,
    );
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    Ok(link)
}
