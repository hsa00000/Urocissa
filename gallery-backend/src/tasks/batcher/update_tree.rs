use crate::operations::open_db::open_data_table;
use crate::public::db::tree::TREE;
use crate::public::structure::response::database_timestamp::DatabaseTimestamp;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_expire::UpdateExpireTask;
use chrono::Utc;
use mini_executor::BatchTask;
use rayon::iter::{ParallelBridge, ParallelIterator};
use rayon::prelude::ParallelSliceMut;
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
    .copied()
    .collect()
});

pub struct UpdateTreeTask;

impl BatchTask for UpdateTreeTask {
    async fn batch_run(_: Vec<Self>) {
        update_tree_task();
    }
}

pub fn update_tree_task() {
    let start_time = Instant::now();
    TREE.with_list_snapshot_update(|| {
        let data_table = open_data_table();

        let priority_list = vec!["DateTimeOriginal", "filename", "modified", "scan_time"];

        let mut database_timestamp_vec: Vec<DatabaseTimestamp> = data_table
            .iter()
            .unwrap()
            .par_bridge()
            .map(|guard| {
                let (_, value) = guard.unwrap();
                let mut abstract_data = value.value();
                // retain only necessary exif data used for query search
                if let Some(exif_vec) = abstract_data.exif_vec_mut() {
                    exif_vec.retain(|k, _| ALLOWED_KEYS.contains(&k.as_str()));
                }
                DatabaseTimestamp::new(abstract_data, &priority_list)
            })
            .collect();

        database_timestamp_vec.par_sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let list_snapshot = crate::public::db::tree::read_tags::TreeListSnapshot::from_records(
            &database_timestamp_vec,
        );
        TREE.replace_tree_snapshot(database_timestamp_vec, list_snapshot);
    });

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = Utc::now().timestamp_millis();
    crate::perf_timing!(
        "tree.rebuild",
        start_time,
        "In-memory cache updated ({}).",
        current_timestamp
    );
}
