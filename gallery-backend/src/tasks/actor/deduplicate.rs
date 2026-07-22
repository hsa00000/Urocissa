use crate::{
    process::{
        artifact_publisher::ArtifactPublisher,
        media_publish::{load_logical_media, publish_media_mutation},
    },
    public::{db::tree::TREE, error_data::handle_error, structure::abstract_data::AbstractData},
};
use anyhow::Result;
use arrayvec::ArrayString;
use mini_executor::Task;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::task::spawn_blocking;
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresignedMetadata {
    pub album_ids: HashSet<ArrayString<64>>,
    pub tags: HashSet<String>,
}

pub struct DeduplicateTask {
    pub path: PathBuf,
    pub hash: ArrayString<64>,
    pub presigned: PresignedMetadata,
}

impl DeduplicateTask {
    pub fn new(path: PathBuf, hash: ArrayString<64>, presigned: PresignedMetadata) -> Self {
        Self {
            path,
            hash,
            presigned,
        }
    }
}

impl Task for DeduplicateTask {
    type Output = Result<Option<AbstractData>>;

    async fn run(self) -> Self::Output {
        spawn_blocking(move || deduplicate_task(&self))
            .await
            .expect("blocking task panicked")
            // convert Err into your crate‑error via `handle_error`
            .map_err(|err| handle_error(err.context("Failed to run deduplicate task")))
    }
}

fn deduplicate_task(task: &DeduplicateTask) -> Result<Option<AbstractData>> {
    let mut abstract_data = AbstractData::new(&task.path, task.hash)?;

    let exists = TREE
        .store
        .read(|reader| Ok::<_, anyhow::Error>(reader.get(task.hash.as_str())?.is_some()))?;

    if exists {
        let file_modify = abstract_data
            .alias_mut()
            .and_then(Vec::pop)
            .ok_or_else(|| anyhow::anyhow!("new duplicate record has no file alias"))?;
        let PresignedMetadata { album_ids, tags } = task.presigned.clone();
        let (slot_ref, _) = load_logical_media(task.hash)?;
        let publisher = ArtifactPublisher::new(format!("deduplicate-{}", Uuid::new_v4()));
        publish_media_mutation(slot_ref, task.hash, publisher, move |latest| {
            latest
                .alias_mut()
                .ok_or_else(|| anyhow::anyhow!("duplicate target is not media"))?
                .push(file_modify);
            latest
                .albums_mut()
                .ok_or_else(|| anyhow::anyhow!("duplicate target is not media"))?
                .extend(album_ids);
            latest.tag_mut().extend(tags);
            Ok(())
        })?;
        warn!("File already exists in the database:\n{:#?}", abstract_data);
        Ok(None)
    } else {
        if let Some(albums) = abstract_data.albums_mut() {
            albums.extend(task.presigned.album_ids.iter().cloned());
        }
        abstract_data
            .tag_mut()
            .extend(task.presigned.tags.iter().cloned());
        Ok(Some(abstract_data))
    }
}
