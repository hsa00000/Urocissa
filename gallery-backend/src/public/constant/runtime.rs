use std::sync::LazyLock;

use rayon::{ThreadPool, ThreadPoolBuilder};
use tokio::runtime::{Builder, Runtime};

pub static CURRENT_NUM_THREADS: LazyLock<usize> = LazyLock::new(|| rayon::current_num_threads());

// Rocket-specific Tokio Runtime
// This runtime is dedicated to handling network requests, with thread names clearly labeled.
pub static ROCKET_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(*CURRENT_NUM_THREADS)
        .thread_name("rocket-io-worker")
        .enable_all()
        .build()
        .expect("Failed to build Rocket Tokio runtime")
});

pub static BATCH_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(*CURRENT_NUM_THREADS)
        .thread_name("tokio-core")
        .enable_all()
        .build()
        .expect("Failed to build Core Tokio runtime")
});

// Background Worker-specific Tokio Runtime
// This runtime handles all non-network asynchronous tasks, such as file monitoring, database initialization, TUI, etc.
pub static INDEX_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(*CURRENT_NUM_THREADS)
        .thread_name("background-task-worker")
        .enable_all()
        .build()
        .expect("Failed to build Worker Tokio runtime")
});

// Rayon thread pool for compute-intensive tasks
// This thread pool is dedicated to CPU-intensive tasks like IndexTask.
// It does not create a global Rayon pool, so it does not interfere with other threads.
pub static WORKER_RAYON_POOL: LazyLock<ThreadPool> = LazyLock::new(|| {
    ThreadPoolBuilder::new()
        .num_threads(*CURRENT_NUM_THREADS)
        .thread_name(|i| format!("cpu-intensive-worker-{}", i))
        .build()
        .expect("Failed to build Worker Rayon pool")
});
