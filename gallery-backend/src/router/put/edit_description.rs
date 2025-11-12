use crate::operations::open_db::{open_data_and_album_tables, open_tree_snapshot_table};
use crate::process::transitor::index_to_abstract_data;
use crate::public::constant::USER_DEFINED_DESCRIPTION;

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
        let (data_table, album_table) = open_data_and_album_tables();
        let tree_snapshot = open_tree_snapshot_table(set_user_defined_description.timestamp)?;

        let mut abstract_data = index_to_abstract_data(
            &tree_snapshot,
            &data_table,
            &album_table,
            set_user_defined_description.index,
        )?;

        match &mut abstract_data {
            AbstractData::Database(db) => {
                db.exif_vec.insert(
                    USER_DEFINED_DESCRIPTION.to_string(),
                    set_user_defined_description
                        .description
                        .clone()
                        .unwrap_or("".to_string()),
                );
            }
            AbstractData::Album(alb) => {
                alb.user_defined_metadata.insert(
                    USER_DEFINED_DESCRIPTION.to_string(),
                    if let Some(desc) = &set_user_defined_description.description {
                        vec![desc.clone()]
                    } else {
                        vec![]
                    },
                );
            }
        }

        BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]));
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
