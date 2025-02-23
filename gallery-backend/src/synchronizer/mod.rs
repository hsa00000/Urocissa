use album::start_album_channel;
use event::start_event_channel;
use rocket::Shutdown;
use video::start_video_channel;
use watch::start_watcher;

use crate::public::{
    expire::EXPIRE, query_snapshot::QUERY_SNAPSHOT, tree::TREE, tree_snapshot::TREE_SNAPSHOT,
};

use futures::stream::{FuturesUnordered, StreamExt};
use log::{error, info};
use std::future::Future;
use tokio::task::JoinError;
use tokio::task::JoinHandle;

pub mod album;
pub mod event;
pub mod video;
pub mod watch;

/// Define a type alias for better readability
type TaskResult = Result<(), JoinError>;

/// Helper function to associate a task with its name
fn named_task(
    name: &'static str,
    handle: JoinHandle<()>,
) -> impl Future<Output = (&'static str, TaskResult)> {
    async move { (name, handle.await) }
}

pub async fn start_sync(shutdown: Shutdown) {
    // Initialize a collection of tasks with their respective names
    let mut tasks = FuturesUnordered::new();

    tasks.push(named_task("Event channel", start_event_channel()));
    tasks.push(named_task("Video channel", start_video_channel()));
    tasks.push(named_task("Album channel", start_album_channel()));
    tasks.push(named_task("Tree loop", TREE.start_loop()));
    tasks.push(named_task("Expire loop", EXPIRE.start_loop()));
    tasks.push(named_task(
        "Query snapshot loop",
        QUERY_SNAPSHOT.start_loop(),
    ));
    tasks.push(named_task(
        "Tree snapshot remove loop",
        TREE_SNAPSHOT.start_loop_remove(),
    ));
    tasks.push(named_task(
        "Tree snapshot flush loop",
        TREE_SNAPSHOT.start_loop_flush(),
    ));
    tasks.push(named_task("Watcher", start_watcher()));

    info!("All channels started.");

    TREE.tree_update();

    // Await the first task to complete
    if let Some((name, result)) = tasks.next().await {
        match result {
            Ok(_) => {
                error!("{} closed unexpectedly.", name);
            }
            Err(err) => {
                error!("{} task failed: {:#?}", name, err);
            }
        }
        // Notify shutdown after any task completes
        shutdown.notify();
    }
}
