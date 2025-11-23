use crate::operations::utils::timestamp::get_current_timestamp_u64;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_expire::UpdateExpireTask;
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
    let priority_list = vec!["DateTimeOriginal", "filename", "modified", "scan_time"];

    let mut database_timestamp_vec: Vec<DatabaseTimestamp> = {
        let databases = TREE.load_all_databases_from_db()?;
        databases
            .into_par_iter()
            .map(|mut db| {
                db.exif_vec
                    .retain(|k, _| ALLOWED_KEYS.contains(&k.as_str()));
                DatabaseTimestamp::new(AbstractData::Database(db.into()), &priority_list)
            })
            .collect()
    };

    let album_vec: Vec<DatabaseTimestamp> = {
        let rows = TREE.read_albums()?;
        rows.into_par_iter()
            .map(|album| DatabaseTimestamp::new(AbstractData::Album(album), &priority_list))
            .collect()
    };

    database_timestamp_vec.extend(album_vec);
    database_timestamp_vec.par_sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    *TREE.in_memory.write().unwrap() = database_timestamp_vec;

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = get_current_timestamp_u64();
    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "In-memory cache updated ({}).", current_timestamp);
    Ok(())
}
