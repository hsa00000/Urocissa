use chrono::Utc;
use serde::{Deserialize, Serialize};

use std::{cmp::Ordering, path::Path};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileModify {
    pub file: String,
    pub modified: i64,
    pub scan_time: i64,
}

impl FileModify {
    pub fn new(file: &Path, modified: i64) -> Self {
        Self {
            file: file.to_string_lossy().into_owned(),
            modified,
            scan_time: Utc::now().timestamp_millis(),
        }
    }
}

impl PartialEq for FileModify {
    fn eq(&self, other: &Self) -> bool {
        self.scan_time == other.scan_time
    }
}
impl Eq for FileModify {}

impl PartialOrd for FileModify {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileModify {
    fn cmp(&self, other: &Self) -> Ordering {
        self.scan_time.cmp(&other.scan_time)
    }
}

impl std::hash::Hash for FileModify {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.scan_time.hash(state);
    }
}
