use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::process::transitor::index_to_abstract_data;
use crate::public::db::tree::TREE;
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::tasks::actor::album::AlbumSelfUpdateTask;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::tasks::{BATCH_COORDINATOR, INDEX_COORDINATOR};
use anyhow::Result;
use arrayvec::ArrayString;
use futures::{StreamExt, TryStreamExt, stream};
use rocket::serde::{Deserialize, json::Json};
use serde::Serialize;
use std::collections::HashSet;

/// Payload for batch editing album associations for multiple items.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditAlbumsData {
    /// List of item indices to modify.
    index_array: Vec<usize>,
    /// List of album IDs to add these items to.
    add_albums_array: Vec<ArrayString<64>>,
    /// List of album IDs to remove these items from.
    remove_albums_array: Vec<ArrayString<64>>,
    /// Snapshot timestamp to ensure data consistency during the read-modify-write cycle.
    timestamp: i64,
}

/// Batches modifications to album associations for multiple media items.
///
/// This performs a read-modify-write operation using a specific DB snapshot
/// to ensure consistency, then triggers background updates for affected albums.
#[put("/put/edit_album", format = "json", data = "<json_data>")]
pub async fn edit_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<EditAlbumsData>,
) -> AppResult<()> {
    const MAX_CONCURRENT_UPDATES: usize = 8;
    let _ = auth?;
    let _ = read_only_mode?;

    let (to_flush, unique_affected_albums) = tokio::task::spawn_blocking(
        move || -> Result<(Vec<_>, Vec<ArrayString<64>>), AppError> {
            let tree_snapshot = open_tree_snapshot_table(json_data.timestamp)
                .or_raise(|| (ErrorKind::Database, "Failed to open tree snapshot"))?;
            let data_table = open_data_table();

            let mut to_flush = Vec::with_capacity(json_data.index_array.len());
            for &index in &json_data.index_array {
                let mut abstract_data = index_to_abstract_data(&tree_snapshot, &data_table, index)
                    .or_raise(|| {
                        (
                            ErrorKind::Database,
                            format!("Failed to retrieve data at index {index}"),
                        )
                    })?;

                if let Some(albums) = abstract_data.albums_mut() {
                    for album_id in &json_data.add_albums_array {
                        albums.insert(*album_id);
                    }
                    for album_id in &json_data.remove_albums_array {
                        albums.remove(album_id);
                    }
                }
                to_flush.push(abstract_data);
            }

            // Deduplicate affected albums to prevent redundant update tasks.
            let unique_affected_albums = json_data
                .add_albums_array
                .iter()
                .chain(json_data.remove_albums_array.iter())
                .copied()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();

            Ok((to_flush, unique_affected_albums))
        },
    )
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(to_flush))
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to execute flush tree task"))?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to execute update tree task"))?;

    stream::iter(unique_affected_albums)
        .map(|album_id| async move {
            INDEX_COORDINATOR
                .execute_waiting(AlbumSelfUpdateTask::new(album_id))
                .await
        })
        .buffer_unordered(MAX_CONCURRENT_UPDATES)
        .try_collect::<Vec<_>>()
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update albums"))?;

    Ok(())
}

/// Payload for updating a specific album's cover image.
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumCover {
    pub album_id: ArrayString<64>,
    /// The hash of the image to set as cover.
    pub cover_hash: ArrayString<64>,
}

/// Updates the cover image of a specific album.
#[put("/put/set_album_cover", data = "<set_album_cover>")]
pub async fn set_album_cover(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    set_album_cover: Json<SetAlbumCover>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;

    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        let set_album_cover_inner = set_album_cover.into_inner();
        let album_id = set_album_cover_inner.album_id;
        let cover_hash = set_album_cover_inner.cover_hash;

        TREE.store.write(|data_table| {
            let album = data_table
                .get(album_id.as_str())?
                .ok_or_else(|| AppError::new(ErrorKind::NotFound, "Album not found"))?
                .value();
            let AbstractData::Album(mut album) = album else {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "Expected Album but got different type",
                ));
            };
            let database = data_table
                .get(cover_hash.as_str())?
                .ok_or_else(|| AppError::new(ErrorKind::NotFound, "Cover image not found"))?
                .value();

            album.set_cover(&database);
            data_table.insert_at(album_id.as_str(), &AbstractData::Album(album))?;
            Ok::<(), AppError>(())
        })?;
        Ok(())
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update tree"))?;
    Ok(())
}

/// Payload for renaming an album.
#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumTitle {
    pub album_id: ArrayString<64>,
    pub title: Option<String>,
}

/// Updates the display title of a specific album.
#[put("/put/set_album_title", data = "<set_album_title>")]
pub async fn set_album_title(
    auth: GuardResult<GuardShare>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    set_album_title: Json<SetAlbumTitle>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;

    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        let set_album_title_inner = set_album_title.into_inner();
        let album_id = set_album_title_inner.album_id;

        TREE.store.write(|data_table| {
            let album = data_table
                .get(album_id.as_str())?
                .ok_or_else(|| AppError::new(ErrorKind::NotFound, "Album not found"))?
                .value();
            let AbstractData::Album(mut album) = album else {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "Expected Album but got different type",
                ));
            };

            album.metadata.title = set_album_title_inner.title;
            data_table.insert_at(album_id.as_str(), &AbstractData::Album(album))?;
            Ok::<(), AppError>(())
        })?;
        Ok(())
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update tree"))?;

    Ok(())
}
