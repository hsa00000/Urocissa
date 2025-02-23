use crate::public::database_struct::database::definition::Database;
use arrayvec::ArrayString;
use dashmap::DashMap;
use futures::TryStreamExt;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use log::error;
use std::sync::Arc;
use tokio::fs;
use tokio::io;

/// Synchronous import function that internally uses Tokio's async capabilities
pub fn import(
    deduplicated_file_list: &DashMap<ArrayString<64>, Database>,
) -> Result<(), Box<dyn std::error::Error>> {
    let progress_bar = ProgressBar::new(deduplicated_file_list.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")? // Added {msg} to the template
            .progress_chars("##-"),
    );

    progress_bar.set_message("Importing...");

    let progress_bar = Arc::new(progress_bar);

    // Create a new Tokio runtime to perform async copy
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    if let Err(e) = rt.block_on(async_import(deduplicated_file_list, progress_bar.clone())) {
        progress_bar.finish_with_message(format!("Import failed"));
        return Err(e);
    }

    progress_bar.set_message(format!("Import completed"));

    progress_bar.finish_with_message(format!("Import completed"));

    Ok(())
}

/// Asynchronous helper function to perform concurrent file copying
async fn async_import(
    deduplicated_file_list: &DashMap<ArrayString<64>, Database>,
    progress_bar: Arc<ProgressBar>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Define the level of concurrency (number of simultaneous operations)
    let concurrency_limit = 10;

    // Create a stream of asynchronous tasks for each file to be copied
    let copy_tasks = stream::iter(deduplicated_file_list.iter())
        .map(|entry| {
            let database = entry.value();

            let source_path = database.source_path();
            let dest_path = database.imported_path();
            let progress_bar = Arc::clone(&progress_bar);
            async move {
                // Ensure the destination directory exists
                if let Some(parent) = dest_path.parent() {
                    if let Err(err) = fs::create_dir_all(parent).await {
                        error!("Failed to create directory {:?}: {:#?}", parent, err);
                        return Err(err);
                    }
                }

                if let Err(err) = fs::copy(&source_path, &dest_path).await {
                    error!(
                        "Failed to copy file from {:?} to {:?}: {:#?}",
                        source_path, dest_path, err
                    );
                    return Err(err);
                }

                progress_bar.inc(1);
                Ok::<(), io::Error>(())
            }
        })
        .buffer_unordered(concurrency_limit); // Limit the number of concurrent tasks

    // Execute all copy tasks concurrently, stopping on the first error
    copy_tasks.try_for_each(|_| async { Ok(()) }).await?;

    Ok(())
}
