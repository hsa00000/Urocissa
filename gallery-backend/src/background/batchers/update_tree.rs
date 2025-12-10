use crate::database::ops::tree::TREE;
use crate::models::entity::abstract_data::AbstractData;
use crate::background::processors::transitor::get_current_timestamp_u64;
use crate::background::actors::BATCH_COORDINATOR;
use crate::background::batchers::expire::UpdateExpireTask;
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

    let mut abstract_data_vec: Vec<AbstractData> = TREE.load_all_data_from_db()?;

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
