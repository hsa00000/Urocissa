use crate::public::db::tree::TREE;
use crate::public::error_data::handle_error;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::object::next_mutation_timestamp;
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
    let _persistence_guard = TREE
        .persistence_lock
        .lock()
        .map_err(|_| anyhow::anyhow!("tree persistence lock poisoned"))?;

    TREE.store
        .write(|data_table| {
            let changed_at = next_mutation_timestamp();
            let album_opt = data_table
                .get(album_id.as_str())?
                .map(|value| value.into_value())
                .and_then(|abstract_data| match abstract_data {
                    AbstractData::Album(album) => Some(album),
                    _ => None,
                });

            if let Some(mut album) = album_opt {
                album.object.pending = true;
                album.self_update(changed_at);
                album.object.pending = false;
                album.object.touch_update_at(changed_at);
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
                    let mut abstract_data = value.into_value();
                    if let Some(albums) = abstract_data.albums_mut() {
                        if albums.remove(&*album_id) {
                            abstract_data.touch_update_at(changed_at);
                        }
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
