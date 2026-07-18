use anyhow::Result;
// use anyhow::anyhow;
use arrayvec::ArrayString;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use redb::ReadOnlyTable;
use rocket::post;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::operations::hash::generate_random_hash;
use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::process::transitor::index_to_abstract_data;
use crate::public::db::tree_snapshot::read_tree_snapshot::MyCow;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::GuardResult;
use crate::tasks::actor::album::AlbumSelfUpdateTask;

use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::album::Album;
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlbum {
    pub title: Option<String>,
    pub elements_index: Vec<usize>,
    pub timestamp: i64,
}

#[post("/post/create_empty_album")]
pub async fn create_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    let album_id = create_album_internal(None).await?;

    Ok(album_id.to_string())
}

#[post("/post/create_non_empty_album", data = "<create_album>")]
pub async fn create_non_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    create_album: Json<CreateAlbum>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    let create_album = create_album.into_inner();
    let album_id = create_album_internal(create_album.title).await?;
    create_album_elements(
        album_id,
        create_album.elements_index,
        create_album.timestamp,
    )
    .await?;

    Ok(album_id.to_string())
}

async fn create_album_internal(title: Option<String>) -> Result<ArrayString<64>, AppError> {
    let start_time = Instant::now();

    let album_id = generate_random_hash();
    let album = Album::new(album_id, title);
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(vec![album.into_abstract_data()]))
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to insert new album"))?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update tree"))?;

    crate::perf_timing!("album.create", start_time, "Create album");
    Ok(album_id)
}

async fn create_album_elements(
    album_id: ArrayString<64>,
    elements_index: Vec<usize>,
    timestamp: i64,
) -> Result<(), AppError> {
    let element_batch =
        tokio::task::spawn_blocking(move || -> Result<Vec<AbstractData>, AppError> {
            let tree_snapshot = open_tree_snapshot_table(timestamp)
                .or_raise(|| (ErrorKind::Database, "Failed to open tree snapshot"))?;
            let data_table = open_data_table();
            elements_index
                .into_par_iter()
                .map(|idx| index_edit_album_insert(&tree_snapshot, &data_table, idx, album_id))
                .collect()
        })
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(element_batch))
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to insert album elements"))?;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update tree"))?;
    BATCH_COORDINATOR
        .execute_waiting(AlbumSelfUpdateTask::new(album_id))
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update album metadata"))??;

    Ok(())
}

pub fn index_edit_album_insert(
    tree_snapshot: &MyCow,
    data_table: &ReadOnlyTable<&'static str, AbstractData>,
    index: usize,
    album_id: ArrayString<64>,
) -> Result<AbstractData, AppError> {
    let mut abstract_data =
        index_to_abstract_data(tree_snapshot, data_table, index).or_raise(|| {
            (
                ErrorKind::Database,
                format!("Failed to convert index {index} to abstract data"),
            )
        })?;
    if let Some(albums) = abstract_data.albums_mut() {
        albums.insert(album_id);
    }
    Ok(abstract_data)
}
