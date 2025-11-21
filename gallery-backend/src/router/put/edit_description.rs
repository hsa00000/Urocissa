use crate::operations::transitor::index_to_hash;
use crate::public::constant::USER_DEFINED_DESCRIPTION;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;

use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Album;
use crate::public::structure::database_struct::database::definition::Database;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use rocket::serde::{Deserialize, json::Json};
use rusqlite::Connection;
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
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&set_user_defined_description.timestamp).unwrap();
        let hash = index_to_hash(&tree_snapshot, set_user_defined_description.index)?;
        let conn = crate::public::db::sqlite::DB_POOL.get().unwrap();
        let mut abstract_data = if let Ok(database) = conn.query_row(
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
