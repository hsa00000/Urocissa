use std::sync::atomic::Ordering;

use arrayvec::ArrayString;
use rocket::serde::{Deserialize, json::Json};

use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::album::Share;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditShare {
    album_id: ArrayString<64>,
    share: Share,
}

#[put("/put/edit_share", format = "json", data = "<json_data>")]
pub async fn edit_share(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<EditShare>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = json_data.into_inner();
    let extra_bytes = data.share.description.capacity()
        + data.share.password.as_ref().map_or(0, String::capacity)
        + std::mem::size_of::<Share>();
    publish_share_change(data.album_id, extra_bytes, |album| {
        album.metadata.share_list.insert(data.share.url, data.share);
        Ok(())
    })
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteShare {
    album_id: ArrayString<64>,
    share_id: ArrayString<64>,
}

#[put("/put/delete_share", format = "json", data = "<json_data>")]
pub async fn delete_share(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<DeleteShare>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = json_data.into_inner();
    publish_share_change(data.album_id, 0, |album| {
        if album.metadata.share_list.remove(&data.share_id).is_none() {
            return Err(AppError::new(ErrorKind::NotFound, "share not found"));
        }
        Ok(())
    })
    .await
}

async fn publish_share_change(
    album_id: ArrayString<64>,
    extra_bytes: usize,
    mutate: impl FnOnce(&mut crate::public::structure::album::AlbumCombined) -> AppResult<()>,
) -> AppResult<()> {
    let reservation = TREE
        .state
        .read()
        .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?
        .albums
        .get(&album_id)
        .map(|album| {
            DirtyOperation::AlbumReplace(album.clone()).estimated_bytes() + extra_bytes + 1_024
        })
        .ok_or_else(|| AppError::new(ErrorKind::NotFound, "album not found"))?;
    WRITE_BEHIND.reserve(reservation).await?;
    let mut state = TREE.state.write().map_err(|_| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::Internal, "tree state lock poisoned")
    })?;
    let album = state.albums.get_mut(&album_id).ok_or_else(|| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::NotFound, "album not found")
    })?;
    if let Err(error) = mutate(album) {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(error);
    }
    let album = album.clone();
    WRITE_BEHIND.enqueue_reserved(DirtyOperation::AlbumReplace(album), reservation);
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    Ok(())
}
