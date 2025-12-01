use crate::workflow::operations::utils::timestamp::get_current_timestamp_u64;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::update_expire::UpdateExpireTask;
use anyhow::Result;
use log::error;
use mini_executor::BatchTask;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Instant;

static ALLOWED_KEYS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "Make",
        "Model",
        "FNumber",
        "ExposureTime",
        "FocalLength",
        "PhotographicSensitivity",
        "DateTimeOriginal",
        "duration",
        "rotation",
    ]
    .iter()
    .cloned()
    .collect()
});

pub struct UpdateTreeTask;

impl BatchTask for UpdateTreeTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            if let Err(e) = update_tree_task() {
                error!("Error in update_tree_task: {}", e);
            }
        }
    }
}

fn update_tree_task() -> Result<()> {
    let start_time = Instant::now();

    let mut abstract_data_vec: Vec<AbstractData> = {
        let databases = TREE.load_all_databases_from_db()?;
        databases
            .into_par_iter()
            .map(|db| AbstractData::DatabaseSchema(db.into()))
            .collect()
    };

    let album_vec: Vec<AbstractData> = {
        let rows = TREE.read_albums()?;
        rows.into_par_iter()
            .map(|album| AbstractData::Album(album))
            .collect()
    };

    abstract_data_vec.extend(album_vec);
    abstract_data_vec.par_sort_by(|a, b| b.compute_timestamp().cmp(&a.compute_timestamp()));

    *TREE.in_memory.write().unwrap() = abstract_data_vec;

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = get_current_timestamp_u64();
    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "In-memory cache updated ({}).", current_timestamp);
    Ok(())
}
