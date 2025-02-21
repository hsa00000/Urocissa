use crate::public::abstract_data::AbstractData;
use crate::public::redb::{ALBUM_TABLE, DATA_TABLE};
use crate::public::tree::TREE;

use arrayvec::ArrayString;
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use redb::ReadableTable;
use std::collections::HashSet;
use std::sync::{Arc, OnceLock};
use tokio;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Notify;

pub struct AlbumQueue {
    pub album_list: Vec<ArrayString<64>>,
    pub notify: Option<Arc<Notify>>,
}

static ALBUM_SELFUPDATE_QUEUE_SENDER: OnceLock<UnboundedSender<AlbumQueue>> = OnceLock::new();
pub fn start_album_channel() -> tokio::task::JoinHandle<()> {
    let (album_selfupdate_queue_sender, mut album_selfupdate_queue_receiver) =
        unbounded_channel::<AlbumQueue>();
    ALBUM_SELFUPDATE_QUEUE_SENDER
        .set(album_selfupdate_queue_sender)
        .unwrap();

    tokio::task::spawn(async move {
        loop {
            let mut buffer = Vec::new();

            album_selfupdate_queue_receiver
                .recv_many(&mut buffer, usize::MAX)
                .await;
            tokio::task::spawn_blocking(move || {
                info!("Perform album self-update");
                let unique_id: HashSet<_> = buffer
                    .iter()
                    .flat_map(|album_queue| album_queue.album_list.iter()) // Flatten all album_list vectors
                    .collect();
                let id_vec: Vec<_> = unique_id.into_iter().collect();
                let txn = TREE.in_disk.begin_write().unwrap();
                {
                    let mut album_table = txn.open_table(ALBUM_TABLE).unwrap();
                    id_vec.into_iter().for_each(|album_id| {
                        let album_opt = album_table
                            .get(&**album_id)
                            .unwrap()
                            .map(|guard| guard.value());

                        match album_opt {
                            Some(mut album) => {
                                album.pending = true;
                                album.self_update();
                                album.pending = false;
                                album_table.insert(&**album_id, album).unwrap();
                            }
                            _ => {
                                // Album has been deleted
                                let ref_data = TREE.in_memory.read().unwrap();

                                // Collect all data contained in this album
                                let hash_list: Vec<_> = ref_data
                                    .par_iter()
                                    .filter_map(|dt| match &dt.abstract_data {
                                        AbstractData::Database(db)
                                            if db.album.contains(&*album_id) =>
                                        {
                                            Some(db.hash)
                                        }
                                        _ => None,
                                    })
                                    .collect();

                                let mut table = txn.open_table(DATA_TABLE).unwrap();

                                // Remove this album from these data
                                hash_list.into_iter().for_each(|hash| {
                                    let mut database = table.get(&*hash).unwrap().unwrap().value();
                                    database.album.remove(&*album_id);
                                    table.insert(&*hash, database).unwrap();
                                });
                            }
                        }
                    });
                }
                txn.commit().unwrap();
                buffer.iter().for_each(|album_queue| {
                    if let Some(notify) = &album_queue.notify {
                        notify.notify_one();
                    }
                });
            })
            .await
            .unwrap();
        }
    })
}
pub fn album_self_update(album_list: Vec<ArrayString<64>>) {
    let album_queue = AlbumQueue {
        album_list: album_list,
        notify: None,
    };
    ALBUM_SELFUPDATE_QUEUE_SENDER
        .get()
        .unwrap()
        .send(album_queue)
        .unwrap();
}
pub async fn album_self_update_async(album_list: Vec<ArrayString<64>>) {
    let notify = Arc::new(Notify::new());
    let album_queue = AlbumQueue {
        album_list: album_list,
        notify: Some(notify.clone()),
    };
    ALBUM_SELFUPDATE_QUEUE_SENDER
        .get()
        .unwrap()
        .send(album_queue)
        .unwrap();
    notify.notified().await
}
