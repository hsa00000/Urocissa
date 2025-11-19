use crate::operations::open_db::open_tree_snapshot_table;
use crate::process::transitor::index_to_abstract_data;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_expire::UpdateExpireTask;
use crate::tasks::BATCH_COORDINATOR;
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
        .execute_batch_waiting(UpdateExpireTask)
        .await?;

    Ok(())
}

fn process_deletes(
    delete_list: Vec<usize>,
    timestamp: u128,
) -> Result<Vec<AbstractData>> {
    let tree_snapshot = open_tree_snapshot_table(timestamp)?;

    let mut abstract_data_to_remove = Vec::new();

    for index in delete_list {
        let abstract_data =
            index_to_abstract_data(&tree_snapshot, index)?;

        abstract_data_to_remove.push(abstract_data);
    }

    Ok(abstract_data_to_remove)
}
