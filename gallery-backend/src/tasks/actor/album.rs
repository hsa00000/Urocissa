use crate::public::db::sqlite::SQLITE;
use crate::public::error_data::handle_error;
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

    let album_opt = SQLITE.get_album(&album_id)?;

    match album_opt {
        Some(mut album) => {
            album.pending = true;
            album.self_update();
            album.pending = false;
            SQLITE.update_album(&album)?;
        }
        None => {
            // Album has been deleted
            let object_ids = SQLITE.get_objects_in_album(&album_id)?;
            for object_id in object_ids {
                SQLITE.remove_album_from_object(&object_id, &album_id)?;
            }
        }
    }
    Ok(())
}
