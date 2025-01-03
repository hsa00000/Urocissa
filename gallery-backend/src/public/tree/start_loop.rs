use super::Tree;
use crate::public::abstract_data::AbstractData;
use crate::public::database_struct::database_timestamp::DataBaseTimestamp;
use crate::public::expire::EXPIRE;
use crate::public::redb::{ALBUM_TABLE, DATA_TABLE};
use crate::router::put::edit_album::AlbumQueue;
use crate::synchronizer::album::ALBUM_SELFUPDATE_QUEUE_SENDER;

use log::info;
use rayon::iter::{ParallelBridge, ParallelIterator};
use rayon::prelude::ParallelSliceMut;
use redb::ReadableTable;
use std::collections::HashSet;
use std::sync::atomic::AtomicU64;
use std::sync::{LazyLock, OnceLock};
use std::time::Instant;
use std::usize;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Notify;

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

pub static ALBUM_WAITING_FOR_MEMORY_UPDATE_SENDER: OnceLock<UnboundedSender<AlbumQueue>> =
    OnceLock::new();

pub static SHOULD_RESET: Notify = Notify::const_new();

pub static VERSION_COUNT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl Tree {
    pub fn start_loop(&self) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn(async {
            let (album_waiting_for_update_sender, mut album_waiting_for_update_receiver) =
                unbounded_channel::<AlbumQueue>();

            ALBUM_WAITING_FOR_MEMORY_UPDATE_SENDER
                .set(album_waiting_for_update_sender)
                .unwrap();

            loop {
                SHOULD_RESET.notified().await;

                let mut buffer = Vec::new();

                if !album_waiting_for_update_receiver.is_empty() {
                    album_waiting_for_update_receiver
                        .recv_many(&mut buffer, usize::MAX)
                        .await;
                }

                tokio::task::spawn_blocking(|| {
                    let start_time = Instant::now();
                    let read_txn = self.in_disk.begin_read().unwrap();
                    let table = read_txn.open_table(DATA_TABLE).unwrap();

                    let priority_list =
                        vec!["DateTimeOriginal", "filename", "modified", "scan_time"];

                    let mut data_vec: Vec<DataBaseTimestamp> = table
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
                            DataBaseTimestamp::new(AbstractData::DataBase(database), &priority_list)
                        })
                        .collect();

                    let album_table = read_txn.open_table(ALBUM_TABLE).unwrap();

                    let album_vec: Vec<DataBaseTimestamp> = album_table
                        .iter()
                        .unwrap()
                        .par_bridge()
                        .map(|guard| {
                            let (_, value) = guard.unwrap();
                            let album = value.value();
                            DataBaseTimestamp::new(AbstractData::Album(album), &priority_list)
                        })
                        .collect();

                    data_vec.extend(album_vec);
                    data_vec.par_sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

                    *self.in_memory.write().unwrap() = data_vec;

                    EXPIRE.update_expire_time(start_time);

                    if !buffer.is_empty() {
                        ALBUM_SELFUPDATE_QUEUE_SENDER
                            .get()
                            .unwrap()
                            .send(
                                buffer
                                    .into_iter()
                                    .map(|album_queue| {
                                        if let Some(notify) = album_queue.notify {
                                            notify.notify_one();
                                        }
                                        album_queue.album_list
                                    })
                                    .flatten()
                                    .collect(),
                            )
                            .unwrap();

                        info!("Send queue albums.");
                    }
                })
                .await
                .unwrap();
            }
        })
    }
}
