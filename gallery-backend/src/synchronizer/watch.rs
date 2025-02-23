use log::info;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use std::thread;
use std::{panic::Location, path::PathBuf};

use crate::public::config::PRIVATE_CONFIG;
use crate::public::error_data::{ErrorData, handle_error};

use super::event::EVENTS_SENDER;
pub fn start_watcher() -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async {
        tokio::task::spawn_blocking(|| {
            let sync_path_list: &Vec<PathBuf> = &PRIVATE_CONFIG.sync_path;
            let mut watcher = get_watcher();

            for path in sync_path_list.iter() {
                watcher.watch(&path, RecursiveMode::Recursive).unwrap();
                info!("Watch path {:?}", path);
            }
            thread::park(); // Because the watcher should keep running
        })
        .await
        .unwrap();
    })
}

fn get_watcher() -> RecommendedWatcher {
    let watcher: RecommendedWatcher =
        notify::recommended_watcher(move |watcher_result: notify::Result<Event>| {
            match watcher_result {
                Ok(wacher_events) => {
                    match wacher_events.kind {
                        EventKind::Create(_) => {
                            if !wacher_events.paths.is_empty() {
                                // Attempt to send the paths without cloning.
                                match EVENTS_SENDER.get().unwrap().send(wacher_events.paths) {
                                    Ok(_) => {
                                        // Successfully sent. Nothing else needed.
                                    }
                                    Err(err) => {
                                        // The send failed, and we get `returned_paths` back here.
                                        let error_data = ErrorData::new(
                                            format!("Failed to send paths: {}", err),
                                            format!(
                                                "Error occurred when sending path: {:?}",
                                                err.0
                                            ),
                                            None,
                                            None,
                                            Location::caller(),
                                            None,
                                        );
                                        handle_error(error_data);
                                    }
                                }
                            }
                        }
                        EventKind::Modify(_) => {
                            // Avoid modifying files within the folder to prevent a full rescan of the entire folder
                            let filtered_paths: Vec<PathBuf> = wacher_events
                                .paths
                                .into_iter()
                                .filter(|path| path.is_file())
                                .collect();

                            if !filtered_paths.is_empty() {
                                EVENTS_SENDER
                                    .get()
                                    .unwrap()
                                    .send(filtered_paths)
                                    .expect("events_sender send error");
                            }
                        }
                        _ => (),
                    }
                }
                Err(err) => error!("watch error: {:#?}", err),
            }
        })
        .unwrap();
    watcher
}
