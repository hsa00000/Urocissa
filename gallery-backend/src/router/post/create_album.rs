use std::collections::BTreeSet;
use std::sync::atomic::Ordering;
use std::time::Instant;

use arrayvec::ArrayString;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

use crate::operations::hash::generate_random_hash;
use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::album::Album;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::selection::{SelectionDescriptor, resolve_selection};
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlbum {
    pub title: Option<String>,
    #[serde(default)]
    pub elements_index: Vec<usize>,
    #[serde(default)]
    pub selection: Option<SelectionDescriptor>,
    pub timestamp: i64,
}

#[post("/post/create_empty_album")]
pub async fn create_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    create_album(None, None).await.map(|id| id.to_string())
}

#[post("/post/create_non_empty_album", data = "<create_album_data>")]
pub async fn create_non_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    create_album_data: Json<CreateAlbum>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = create_album_data.into_inner();
    let selection = data
        .selection
        .unwrap_or_else(|| SelectionDescriptor::explicit(data.elements_index));
    let resolved =
        tokio::task::spawn_blocking(move || resolve_selection(data.timestamp, &selection))
            .await
            .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;
    create_album(data.title, Some(resolved.targets))
        .await
        .map(|id| id.to_string())
}

async fn create_album(
    title: Option<String>,
    members: Option<crate::public::db::tree::state::TargetSet>,
) -> AppResult<ArrayString<64>> {
    let started = Instant::now();
    let album_id = generate_random_hash();
    let album_data = Album::new(album_id, title).into_abstract_data();
    let initial_album = match &album_data {
        crate::public::structure::abstract_data::AbstractData::Album(album) => album.clone(),
        _ => unreachable!(),
    };
    let mut membership_operation = members.map(|targets| DirtyOperation::Albums {
        targets,
        add: BTreeSet::from([album_id]),
        remove: BTreeSet::new(),
    });
    let reservation = 4_096
        + membership_operation
            .as_ref()
            .map_or(0, |operation| operation.estimated_bytes() + 4_096);
    WRITE_BEHIND.reserve(reservation).await?;
    let mut state = TREE.state.write().map_err(|_| {
        WRITE_BEHIND.release_reservation(reservation);
        AppError::new(ErrorKind::Internal, "tree state lock poisoned")
    })?;
    if membership_operation.as_ref().is_some_and(|operation| {
        matches!(operation, DirtyOperation::Albums { targets, .. } if !targets.is_current(&state))
    }) {
        WRITE_BEHIND.release_reservation(reservation);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection became stale before album publication",
        ));
    }
    state.insert(&album_data);
    if let Some(DirtyOperation::Albums { targets, .. }) = &mut membership_operation {
        *targets = state.media_targets(targets);
    }
    if let Some(DirtyOperation::Albums { targets, add, .. }) = &membership_operation {
        state.edit_album_memberships(targets.ordinals(), add, &BTreeSet::new());
    }
    let album = state
        .albums
        .get(&album_id)
        .cloned()
        .expect("new album must be in catalog");
    WRITE_BEHIND.enqueue_reserved(DirtyOperation::AlbumCreate(initial_album), reservation);
    if let Some(operation) = membership_operation {
        WRITE_BEHIND.enqueue_reserved(operation, 0);
        WRITE_BEHIND.enqueue_reserved(DirtyOperation::AlbumReplace(album), 0);
    }
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    crate::perf_timing!("album.create.ram_publish", started, "Create album in RAM");
    Ok(album_id)
}
