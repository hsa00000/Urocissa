use crate::public::db::tree::TREE;
use crate::public::error_data::handle_error;
use crate::public::structure::abstract_data::AbstractData;
use anyhow::Context;
use anyhow::Result;
use arrayvec::ArrayString;
use log::info;
use mini_executor::Task;
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

    async fn run(self) -> Self::Output {
        spawn_blocking(move || album_task(self.album_id))
            .await
            .expect("blocking task panicked")
            .map_err(|err| handle_error(err.context("Failed to run album task")))
    }
}

pub fn album_task(album_id: ArrayString<64>) -> Result<()> {
    info!("Perform album self-update");
    let album_id_for_cache = album_id;

    TREE.store
        .write(|data_table| {
            let album_opt = data_table
                .get(album_id.as_str())?
                .map(|value| value.value())
                .and_then(|abstract_data| match abstract_data {
                    AbstractData::Album(album) => Some(album),
                    _ => None,
                });

            if let Some(mut album) = album_opt {
                album.object.pending = true;
                album.self_update();
                album.object.pending = false;
                data_table.insert(&AbstractData::Album(album))?;
            } else {
                // Album has been deleted
                let state = TREE.state.read().unwrap();
                let hash_list = state
                    .query
                    .albums
                    .get(&album_id)
                    .into_iter()
                    .flat_map(|members| members.iter())
                    .filter_map(|ordinal| state.slot_for_ordinal(ordinal))
                    .filter_map(|slot_ref| state.get(slot_ref))
                    .map(|record| record.id)
                    .collect::<Vec<_>>();
                drop(state);

                // Remove this album from these data
                for hash in hash_list {
                    let Some(value) = data_table.get(hash.as_str())? else {
                        continue;
                    };
                    let mut abstract_data = value.value();
                    if let Some(albums) = abstract_data.albums_mut() {
                        albums.remove(&*album_id);
                    }
                    data_table.insert(&abstract_data)?;
                }
            }
            Ok::<(), anyhow::Error>(())
        })
        .context("album transaction failed")?;
    TREE.refresh_album_snapshot(album_id_for_cache.as_str())
        .context("album cache update failed")?;
    Ok(())
}
