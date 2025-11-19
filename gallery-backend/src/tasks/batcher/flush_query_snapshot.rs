use crate::public::db::query_snapshot::QUERY_SNAPSHOT;
use mini_executor::BatchTask;

pub struct FlushQuerySnapshotTask;

impl BatchTask for FlushQuerySnapshotTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            flush_query_snapshot_task();
        }
    }
}

fn flush_query_snapshot_task() {
    // We are removing Redb, so we just clear the in-memory cache to prevent memory leaks.
    // TODO: Implement a proper in-memory LRU cache or SQLite-based cache if needed.
    QUERY_SNAPSHOT.in_memory.clear();
}

