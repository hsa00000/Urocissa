use crate::operations::transitor::index_to_hash;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Album;
use crate::public::structure::database_struct::database::definition::Database;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::tasks::actor::album::AlbumSelfUpdateTask;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::tasks::{BATCH_COORDINATOR, INDEX_COORDINATOR};
use anyhow::Result;
use arrayvec::ArrayString;
use futures::future::try_join_all;
use rocket::serde::{Deserialize, json::Json};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteList {
    delete_list: Vec<usize>,
    timestamp: u128,
}
#[delete("/delete/delete-data", format = "json", data = "<json_data>")]
pub async fn delete_data(
    auth: GuardResult<GuardAuth>,
    _read_only_mode: GuardReadOnlyMode,
    json_data: Json<DeleteList>,
) -> AppResult<()> {
    let _ = auth?;
    let (abstract_data_to_remove, all_affected_album_ids) = tokio::task::spawn_blocking({
        let delete_list = json_data.delete_list.clone();
        let timestamp = json_data.timestamp;
        move || process_deletes(delete_list, timestamp)
    })
    .await??;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::remove(abstract_data_to_remove))
        .await?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;

    try_join_all(
        all_affected_album_ids
            .into_iter()
            .map(|album_id| async move {
                INDEX_COORDINATOR
                    .execute_waiting(AlbumSelfUpdateTask::new(album_id))
                    .await
            }),
    )
    .await?;
    Ok(())
}

fn process_deletes(
    delete_list: Vec<usize>,
    timestamp: u128,
) -> Result<(Vec<AbstractData>, Vec<ArrayString<64>>)> {
    let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&timestamp).unwrap();
    let conn = crate::public::db::sqlite::DB_POOL.get().unwrap();

    let mut all_affected_album_ids = Vec::new();
    let mut abstract_data_to_remove = Vec::new();

    for index in delete_list {
        let hash = index_to_hash(&tree_snapshot, index)?;
        let abstract_data = if let Ok(database) = conn.query_row(
            "SELECT * FROM database WHERE hash = ?",
            [&*hash],
            |row| Database::from_row(row)
        ) {
            AbstractData::Database(database)
        } else if let Ok(album) = conn.query_row(
            "SELECT * FROM album WHERE id = ?",
            [&*hash],
            |row| Album::from_row(row)
        ) {
            AbstractData::Album(album)
        } else {
            return Err(anyhow::anyhow!("No data found for hash: {}", hash));
        };

        let affected_albums = match &abstract_data {
            AbstractData::Database(db) => db.album.iter().cloned().collect(),
            AbstractData::Album(album) => vec![album.id],
        };

        all_affected_album_ids.extend(affected_albums);
        abstract_data_to_remove.push(abstract_data);
    }

    Ok((abstract_data_to_remove, all_affected_album_ids))
}
