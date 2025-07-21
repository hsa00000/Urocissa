use crate::operations::open_db::{open_data_and_album_tables, open_tree_snapshot_table};
use crate::process::transitor::index_to_abstract_data;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::COORDINATOR;
use crate::tasks::actor::album::AlbumSelfUpdateTask;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use arrayvec::ArrayString;
use futures::future::join_all;
use log::error;
use rocket::serde::{Deserialize, json::Json};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteList {
    delete_list: Vec<usize>,
    timestamp: u128,
}
#[delete("/delete/delete-data", format = "json", data = "<json_data>")]
pub async fn delete_data(
    _auth: GuardAuth,
    _read_only_mode: GuardReadOnlyMode,
    json_data: Json<DeleteList>,
) -> AppResult<()> {
    let (abstract_data_to_remove, all_affected_album_ids) = tokio::task::spawn_blocking({
        let delete_list = json_data.delete_list.clone();
        let timestamp = json_data.timestamp;
        move || process_deletes(delete_list, timestamp)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Blocking task failed: {}", e))??;

    COORDINATOR
        .execute_batch_waiting(FlushTreeTask::remove(abstract_data_to_remove))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to flush tree: {}", e))?;

    COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update tree: {}", e))?;

    let album_futures = all_affected_album_ids
        .into_iter()
        .map(|album_id| async move {
            if let Err(e) = COORDINATOR
                .execute_waiting(AlbumSelfUpdateTask::new(album_id))
                .await
            {
                error!("Failed to process album: {}", e);
            }
        });

    join_all(album_futures).await;
    Ok(())
}

fn process_deletes(
    delete_list: Vec<usize>,
    timestamp: u128,
) -> Result<(Vec<AbstractData>, Vec<ArrayString<64>>)> {
    let (data_table, album_table) = open_data_and_album_tables();
    let tree_snapshot = open_tree_snapshot_table(timestamp)?;

    let mut all_affected_album_ids = Vec::new();
    let mut abstract_data_to_remove = Vec::new();

    for index in delete_list {
        let abstract_data =
            index_to_abstract_data(&tree_snapshot, &data_table, &album_table, index)?;

        let affected_albums = match &abstract_data {
            AbstractData::Database(db) => db.album.iter().cloned().collect(),
            AbstractData::Album(album) => vec![album.id],
        };

        all_affected_album_ids.extend(affected_albums);
        abstract_data_to_remove.push(abstract_data);
    }

    Ok((abstract_data_to_remove, all_affected_album_ids))
}
