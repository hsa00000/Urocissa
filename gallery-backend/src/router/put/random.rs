use crate::public::structure::abstract_data::AbstractData;
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::{
    public::structure::database_struct::database::definition::Database,
    tasks::batcher::flush_tree::FlushTreeTask,
};
use anyhow::Result;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
#[get("/put/generate_random_data?<number>")]
pub async fn generate_random_data(
    auth: Result<GuardAuth>,
    _read_only_mode: GuardReadOnlyMode,
    number: usize,
) -> AppResult<()> {
    let _ = auth?;
    let database_list: Vec<AbstractData> = (0..number)
        .into_par_iter()
        .map(|_| Database::generate_random_data())
        .map(|database| AbstractData::Database(database))
        .collect();
    BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(database_list));
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update tree: {}", e))?;
    info!("Insert random data complete");
    Ok(())
}
