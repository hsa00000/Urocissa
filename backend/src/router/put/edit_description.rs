use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::operations::transitor::index_to_hash;
use crate::public::structure::abstract_data::AbstractData;

use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use rocket::serde::{Deserialize, json::Json};
use serde::Serialize;

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetUserDefinedDescription {
    pub index: usize,
    pub description: Option<String>,
    pub timestamp: u128,
}

#[put(
    "/put/set_user_defined_description",
    data = "<set_user_defined_description>"
)]
pub async fn set_user_defined_description(
    auth: GuardResult<GuardShare>,
    read_only_mode: Result<GuardReadOnlyMode>,
    set_user_defined_description: Json<SetUserDefinedDescription>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || -> Result<()> {
        let data_table = open_data_table();
        let tree_snapshot = open_tree_snapshot_table(set_user_defined_description.timestamp)?;

        let hash = index_to_hash(&tree_snapshot, set_user_defined_description.index)?;

        if let Some(guard) = data_table.get(&*hash).unwrap() {
            let mut abstract_data = guard.value();

            match &mut abstract_data {
                AbstractData::Image(img) => {
                    img.object.description = set_user_defined_description.description.clone();
                }
                AbstractData::Video(vid) => {
                    vid.object.description = set_user_defined_description.description.clone();
                }
                AbstractData::Album(album) => {
                    album.object.description = set_user_defined_description.description.clone();
                }
            }

            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]));
        }

        Ok(())
    })
    .await
    .unwrap()?;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(())
}
