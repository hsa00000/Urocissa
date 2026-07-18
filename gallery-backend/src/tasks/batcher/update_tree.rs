use crate::operations::open_db::open_data_table;
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::TreeState;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_expire::UpdateExpireTask;
use chrono::Utc;
use mini_executor::BatchTask;
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

pub fn update_tree_task() -> (usize, usize) {
    let start_time = Instant::now();
    let counts = TREE.with_list_snapshot_update(|| {
        let data_table = open_data_table();

        let records = data_table.iter().unwrap().map(|guard| {
            let (_, value) = guard.unwrap();
            let mut abstract_data = value.into_value();
            // retain only necessary exif data used for query search
            if let Some(exif_vec) = abstract_data.exif_vec_mut() {
                exif_vec.retain(|k, _| ALLOWED_KEYS.contains(&k.as_str()));
            }
            abstract_data
        });
        let state = TreeState::from_records(records);
        let counts = (state.len(), state.albums.len());
        TREE.replace_tree_snapshot(
            state,
            crate::public::db::tree::read_tags::TreeListSnapshot::default(),
        );
        counts
    });

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = Utc::now().timestamp_millis();
    crate::perf_timing!(
        "tree.rebuild",
        start_time,
        "In-memory cache updated ({}).",
        current_timestamp
    );
    counts
}
