use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::workflow::processors::transitor::index_to_hash;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::flush_tree::FlushTreeTask;
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
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
    let abstract_data_to_remove = tokio::task::spawn_blocking({
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

    // Album stats are now updated by database triggers
    Ok(())
}

fn process_deletes(delete_list: Vec<usize>, timestamp: u128) -> Result<Vec<AbstractData>> {
    let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&timestamp).unwrap();

    let mut abstract_data_to_remove = Vec::new();

    for index in delete_list {
        let hash = index_to_hash(&tree_snapshot, index)?;
        let abstract_data = TREE.load_from_db(&hash)?;

        abstract_data_to_remove.push(abstract_data);
    }

    Ok(abstract_data_to_remove)
}
