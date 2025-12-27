use crate::public::constant::redb::DATA_TABLE;
use crate::public::db::tree::TREE;
use crate::public::error_data::handle_error;
use crate::public::structure::abstract_data::AbstractData;
use anyhow::Context;
use anyhow::Result;
use arrayvec::ArrayString;
use log::info;
use mini_executor::Task;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use redb::ReadableTable;
use tokio::task::spawn_blocking;

pub struct AlbumSelfUpdateTask {
    album_id: ArrayString<64>,
}

impl AlbumSelfUpdateTask {
    pub fn new(album_id: ArrayString<64>) -> Self {
        Self { album_id }
    }
}

impl Task for AlbumSelfUpdateTask {
    type Output = Result<()>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || album_task(self.album_id))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run album task")))
        }
    }
}

pub fn album_task(album_id: ArrayString<64>) -> Result<()> {
    info!("Perform album self-update");

    let txn = TREE
        .in_disk
        .begin_write()
        .context("begin_write failed (album)")?;
    {
        let mut data_table = txn.open_table(DATA_TABLE)?;

        let album_opt = data_table
            .get(&*album_id)
            .unwrap()
            .and_then(|guard| {
                let abstract_data = guard.value();
                match abstract_data {
                    AbstractData::Album(album) => Some(album),
                    _ => None,
                }
            });

        match album_opt {
            Some(mut album) => {
                album.object.pending = true;
                album.self_update();
                album.object.pending = false;
                data_table.insert(&*album_id, AbstractData::Album(album)).unwrap();
            }
            _ => {
                // Album has been deleted
                let ref_data = TREE.in_memory.read().unwrap();

                // Collect all data contained in this album
                let hash_list: Vec<_> = ref_data
                    .par_iter()
                    .filter_map(|dt| match &dt.abstract_data {
                        AbstractData::Image(img) if img.metadata.albums.contains(&*album_id) => {
                            Some(img.object.id)
                        }
                        AbstractData::Video(vid) if vid.metadata.albums.contains(&*album_id) => {
                            Some(vid.object.id)
                        }
                        _ => None,
                    })
                    .collect();

                // Remove this album from these data
                hash_list.into_iter().for_each(|hash| {
                    let mut abstract_data = data_table.get(&*hash).unwrap().unwrap().value();
                    if let Some(albums) = abstract_data.albums_mut() {
                        albums.remove(&*album_id);
                    }
                    data_table.insert(&*hash, abstract_data).unwrap();
                });
            }
        }
    }
    txn.commit().context("commit failed (album)")?;
    Ok(())
}
