use bitcode::{Decode, Encode};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use std::{cmp::Ordering, path::Path};

#[derive(Debug, Default, Clone, Deserialize, Serialize, Decode, Encode, Hash)]
#[serde(rename_all = "camelCase")]
pub struct FileModify {
    pub file: String,
    pub modified: u128,
    pub scan_time: u128,
}

impl FileModify {
    pub fn new(file: &Path, modified: u128) -> Self {
        Self {
            file: file.to_string_lossy().into_owned(),
            modified,
            scan_time: Utc::now().timestamp_millis() as u128,
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
