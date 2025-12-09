use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::workflow::processors::transitor::index_to_hash;

use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::flush_tree::{FlushOperation, FlushTreeTask};
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
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
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&set_user_defined_description.timestamp)
            .unwrap();
        let hash = index_to_hash(&tree_snapshot, set_user_defined_description.index)?;
        let abstract_data = TREE.load_from_db(&hash)?;

        let mut operations = Vec::new();
        let new_desc = set_user_defined_description.description.clone();

        match abstract_data {
            AbstractData::Image(mut i) => {
                i.object.description = new_desc;
                operations.push(FlushOperation::InsertAbstractData(AbstractData::Image(i)));
            }
            AbstractData::Video(mut v) => {
                v.object.description = new_desc;
                operations.push(FlushOperation::InsertAbstractData(AbstractData::Video(v)));
            }
            AbstractData::Album(mut alb) => {
                alb.object.description = new_desc;
                operations.push(FlushOperation::InsertAbstractData(AbstractData::Album(alb)));
            }
        }

        if !operations.is_empty() {
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask { operations });
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
