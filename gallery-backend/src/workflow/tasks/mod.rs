use crate::public::constant::runtime::{BATCH_RUNTIME, INDEX_RUNTIME};
use mini_executor::TaskExecutor;
use std::sync::LazyLock;

pub mod actor;
pub mod batcher;
pub mod looper;

pub static INDEX_COORDINATOR: LazyLock<TaskExecutor> =
    LazyLock::new(|| TaskExecutor::new(&INDEX_RUNTIME));

pub static BATCH_COORDINATOR: LazyLock<TaskExecutor> =
    LazyLock::new(|| TaskExecutor::new(&BATCH_RUNTIME));
