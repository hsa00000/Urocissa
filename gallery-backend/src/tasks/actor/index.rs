use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio_rayon::AsyncThreadPool;

use crate::public::constant::runtime::WORKER_RAYON_POOL;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::relations::exif_databases::ExifSchema;

use crate::{
    process::info::{process_image_info, process_video_info},
    public::{
        constant::VALID_IMAGE_EXTENSIONS,
        error_data::handle_error,
        structure::{database::definition::DatabaseSchema, guard::PendingGuard},
        tui::{DASHBOARD, FileType},
    },
    tasks::batcher::flush_tree::{FlushOperation, FlushTreeTask},
};
use mini_executor::Task;

pub struct IndexTask {
    pub path: PathBuf,
    pub database: DatabaseSchema,
}

impl IndexTask {
    pub fn new(path: PathBuf, database: DatabaseSchema) -> Self {
        Self { path, database }
    }
}

impl Task for IndexTask {
    type Output = Result<(DatabaseSchema, FlushTreeTask)>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            WORKER_RAYON_POOL
                .spawn_async(move || index_task_match(self.database, &self.path))
                .await
                .map_err(|err| handle_error(err.context("Failed to run index task")))
        }
    }
}

/// Outer layer: unify business result matching and update TUI  
/// (success -> advance, failure -> mark_failed)
fn index_task_match(
    database: DatabaseSchema,
    path: &Path,
) -> Result<(DatabaseSchema, FlushTreeTask)> {
    let hash = database.hash; // hash is Copy, no need to clone
    match index_task(database, path) {
        Ok((db, task)) => {
            DASHBOARD.advance_task_state(&hash);
            Ok((db, task))
        }
        Err(e) => {
            DASHBOARD.mark_failed(&hash);
            Err(e)
        }
    }
}

/// Inner layer: only responsible for business logic, no TUI state updates
fn index_task(
    mut database: DatabaseSchema,
    path: &Path,
) -> Result<(DatabaseSchema, FlushTreeTask)> {
    let hash = database.hash;
    let newest_path = path.to_string_lossy().to_string();

    // Register task in dashboard; attach context if extension is invalid
    DASHBOARD.add_task(
        hash,
        newest_path.clone(),
        FileType::try_from(database.ext_type.as_str())
            .context(format!("unsupported file type: {}", database.ext_type))?,
    );

    // Branch processing based on file extension
    let is_image = VALID_IMAGE_EXTENSIONS.contains(&database.ext.as_str());
    let exif_vec = if is_image {
        let exif_vec = process_image_info(&mut database, path).context(format!(
            "failed to process image metadata pipeline:\n{:#?}",
            database
        ))?;
        exif_vec
    } else {
        let exif_vec = process_video_info(&mut database).context(format!(
            "failed to process video metadata pipeline:\n{:#?}",
            database
        ))?;
        database.pending = true;
        exif_vec
    };

    let abstract_data = AbstractData::DatabaseSchema(database.clone().into());
    let mut operations = vec![FlushOperation::InsertAbstractData(abstract_data)];

    // Insert EXIF data
    let hash_str = database.hash.as_str();
    for (tag, value) in &exif_vec {
        operations.push(FlushOperation::InsertExif(
            crate::public::structure::relations::exif_databases::ExifSchema {
                hash: hash_str.to_string(),
                tag: tag.clone(),
                value: value.clone(),
            },
        ));
    }

    let flush_task = FlushTreeTask { operations };

    Ok((database, flush_task))
}
