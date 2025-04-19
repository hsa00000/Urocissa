use super::Tree;
use crate::public::abstract_data::AbstractData;
use crate::public::database_struct::database_timestamp::DatabaseTimestamp;
use crate::public::expire::EXPIRE;
use crate::public::utils::start_loop_util;

use rayon::iter::{ParallelBridge, ParallelIterator};
use rayon::prelude::ParallelSliceMut;
use redb::ReadableTable;
use std::collections::HashSet;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::Instant;
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedSender;

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

static TREE_UPDATE_SENDER: OnceLock<UnboundedSender<Option<Arc<Notify>>>> = OnceLock::new();

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl Tree {
    pub fn start_loop(&'static self) -> tokio::task::JoinHandle<()> {
        start_loop_util(&TREE_UPDATE_SENDER, |buffer| {
            let start_time = Instant::now();
            let table = self.api_read_tree();

            let priority_list = vec!["DateTimeOriginal", "filename", "modified", "scan_time"];

            let mut data_vec: Vec<DatabaseTimestamp> = table
                .iter()
                .unwrap()
                .par_bridge()
                .map(|guard| {
                    let (_, value) = guard.unwrap();
                    let mut database = value.value();
                    // retain only necessary exif data used for query search
                    database
                        .exif_vec
                        .retain(|k, _| ALLOWED_KEYS.contains(&k.as_str()));
                    DatabaseTimestamp::new(AbstractData::Database(database), &priority_list)
                })
                .collect();

            let album_table = self.api_read_album();

            let album_vec: Vec<DatabaseTimestamp> = album_table
                .iter()
                .unwrap()
                .par_bridge()
                .map(|guard| {
                    let (_, value) = guard.unwrap();
                    let album = value.value();
                    DatabaseTimestamp::new(AbstractData::Album(album), &priority_list)
                })
                .collect();

            data_vec.extend(album_vec);
            data_vec.par_sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

            *self.in_memory.write().unwrap() = data_vec;

            EXPIRE.update_expire_time(start_time);
            buffer.into_iter().for_each(|notify_opt| {
                if let Some(notify) = notify_opt {
                    notify.notify_one()
                };
            });
        })
    }
    pub fn tree_update(&self) {
        TREE_UPDATE_SENDER.get().unwrap().send(None).unwrap();
    }
    pub async fn should_update_async(&self) {
        let notify = Arc::new(Notify::new());
        TREE_UPDATE_SENDER
            .get()
            .unwrap()
            .send(Some(notify.clone()))
            .unwrap();
        notify.notified().await
    }
}
