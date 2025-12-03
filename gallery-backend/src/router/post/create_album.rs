use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::GuardResult;
use crate::workflow::processors::transitor::index_to_hash;
use anyhow::Result;
use arrayvec::ArrayString;
use rand::{Rng, distr::Alphanumeric};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rocket::post;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::table::album::AlbumCombined;
use crate::table::meta_album::AlbumMetadataSchema;
use crate::table::object::ObjectSchema;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::flush_tree::FlushTreeTask;
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlbum {
    pub title: Option<String>,
    pub elements_index: Vec<usize>,
    pub timestamp: u128,
}

#[post("/post/create_empty_album")]
pub async fn create_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    let album_id = create_album_internal(None).await?;

    Ok(album_id.to_string())
}

#[post("/post/create_non_empty_album", data = "<create_album>")]
pub async fn create_non_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
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

async fn create_album_internal(title: Option<String>) -> Result<ArrayString<64>> {
    let start_time = Instant::now();

    let album_id = generate_random_hash();
    let object = ObjectSchema::new(album_id, "album");
    let metadata = AlbumMetadataSchema::new(album_id, title);
    let album = AbstractData::Album(AlbumCombined { object, metadata });
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(vec![album]))
        .await?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;

    info!(duration = &*format!("{:?}", start_time.elapsed()); "Create album");
    Ok(album_id)
}

async fn create_album_elements(
    album_id: ArrayString<64>,
    elements_index: Vec<usize>,
    timestamp: u128,
) -> Result<()> {
    let element_batch = tokio::task::spawn_blocking(move || -> Result<Vec<AbstractData>> {
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&timestamp).unwrap();
        elements_index
            .into_par_iter()
            .map(|idx| index_edit_album_insert(&tree_snapshot, idx, album_id))
            .collect()
    })
    .await??;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(element_batch))
        .await?;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;
    // Album stats are now updated by database triggers

    Ok(())
}

pub fn index_edit_album_insert(
    tree_snapshot: &crate::public::db::tree_snapshot::read_tree_snapshot::MyCow,
    database_index: usize,
    album_id: ArrayString<64>,
) -> Result<AbstractData> {
    let hash = index_to_hash(tree_snapshot, database_index)?;
    let data_opt = TREE.load_data_from_hash(&hash)?;
    let mut data = data_opt.ok_or_else(|| anyhow::anyhow!("Data not found for hash: {}", hash))?;
    
    match &mut data {
        AbstractData::Image(i) => { i.albums.insert(album_id); },
        AbstractData::Video(v) => { v.albums.insert(album_id); },
        _ => {}
    }
    
    Ok(data)
}

/// Generate a random 64-character lowercase alphanumeric hash
pub fn generate_random_hash() -> ArrayString<64> {
    let hash: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .take(64)
        .map(char::from)
        .collect();

    ArrayString::<64>::from(&hash).unwrap()
}
