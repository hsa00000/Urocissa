use crate::operations::utils::timestamp::get_current_timestamp_u64;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Album;
use crate::public::structure::database_struct::database::definition::Database;
use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_expire::UpdateExpireTask;
use mini_executor::BatchTask;
use rayon::prelude::*;
use rusqlite::Connection;
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
            update_tree_task();
        }
    }
}

fn update_tree_task() {
    let start_time = Instant::now();
    let conn = crate::public::db::sqlite::DB_POOL.get().expect("Failed to open database");

    let priority_list = vec!["DateTimeOriginal", "filename", "modified", "scan_time"];

    let mut database_timestamp_vec: Vec<DatabaseTimestamp> = {
        let mut stmt = conn.prepare("SELECT * FROM database").unwrap();
        let rows: Vec<Database> = stmt
            .query_map([], Database::from_row)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        rows.into_par_iter()
            .map(|mut db| {
                db.exif_vec
                    .retain(|k, _| ALLOWED_KEYS.contains(&k.as_str()));
                DatabaseTimestamp::new(AbstractData::Database(db), &priority_list)
            })
            .collect()
    };

    let album_vec: Vec<DatabaseTimestamp> = {
        let mut stmt = conn.prepare("SELECT * FROM album").unwrap();
        let rows: Vec<Album> = stmt
            .query_map([], Album::from_row)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
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
}
