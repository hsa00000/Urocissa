pub mod file_ops;
pub mod deduplicate;
pub mod hash;
pub mod indexer;
pub mod video_transcode;

pub use file_ops::copy;
pub use file_ops::read as open_file;
pub use file_ops::delete as delete_in_update;
pub use indexer as index;
pub use video_transcode as video;

use mini_executor::TaskExecutor;
use std::sync::LazyLock;
// 引入 runtime 常數
use crate::common::{BATCH_RUNTIME, INDEX_RUNTIME};

pub static BATCH_COORDINATOR: LazyLock<TaskExecutor> = LazyLock::new(|| TaskExecutor::new(&BATCH_RUNTIME));
pub static INDEX_COORDINATOR: LazyLock<TaskExecutor> = LazyLock::new(|| TaskExecutor::new(&INDEX_RUNTIME));
