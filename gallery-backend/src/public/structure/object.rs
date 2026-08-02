#![allow(clippy::struct_excessive_bools)]
use arrayvec::ArrayString;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicI64, Ordering};

static LAST_MUTATION_TIMESTAMP: AtomicI64 = AtomicI64::new(0);

pub fn observe_mutation_timestamp(timestamp: i64) {
    LAST_MUTATION_TIMESTAMP.fetch_max(timestamp, Ordering::Relaxed);
}

pub fn next_mutation_timestamp() -> i64 {
    let now = Utc::now().timestamp_millis();
    let mut observed = LAST_MUTATION_TIMESTAMP.load(Ordering::Relaxed);
    loop {
        let next = now.max(observed.saturating_add(1));
        match LAST_MUTATION_TIMESTAMP.compare_exchange_weak(
            observed,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return next,
            Err(current) => observed = current,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObjectType {
    Image,
    Video,
    Album,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Image => write!(f, "image"),
            ObjectType::Video => write!(f, "video"),
            ObjectType::Album => write!(f, "album"),
        }
    }
}

impl FromStr for ObjectType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image" => Ok(ObjectType::Image),
            "video" => Ok(ObjectType::Video),
            "album" => Ok(ObjectType::Album),
            _ => Err(format!("Invalid ObjectType: {s}")),
        }
    }
}

/// Common object schema shared between Image, Video, and Album
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSchema {
    pub id: ArrayString<64>,
    pub obj_type: ObjectType,
    pub pending: bool,
    pub thumbhash: Option<Vec<u8>>,
    #[serde(default)]
    pub cache_version: u32,
    pub description: Option<String>,
    pub tags: HashSet<String>,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub is_trashed: bool,
    pub update_at: i64,
}

impl ObjectSchema {
    pub fn new(id: ArrayString<64>, obj_type: ObjectType) -> Self {
        let update_at = Utc::now().timestamp_millis();
        observe_mutation_timestamp(update_at);
        Self {
            id,
            obj_type,
            pending: false,
            thumbhash: None,
            cache_version: 0,
            description: None,
            tags: HashSet::new(),
            is_favorite: false,
            is_archived: false,
            is_trashed: false,
            update_at,
        }
    }

    pub fn touch_update_at(&mut self, changed_at: i64) {
        self.update_at = self.update_at.max(changed_at);
        observe_mutation_timestamp(self.update_at);
    }
}

#[cfg(test)]
mod tests {
    use arrayvec::ArrayString;

    use super::{ObjectSchema, ObjectType, next_mutation_timestamp};

    #[test]
    fn mutation_timestamp_is_newer_than_observed_object_times() {
        let mut object = ObjectSchema::new(
            ArrayString::<64>::from("timestamp-object").unwrap(),
            ObjectType::Image,
        );
        let changed_at = next_mutation_timestamp();
        assert!(changed_at > object.update_at);

        object.touch_update_at(changed_at);
        assert_eq!(object.update_at, changed_at);
    }

    #[test]
    fn cache_version_uses_the_camel_case_json_field() {
        let object = ObjectSchema::new(
            ArrayString::<64>::from("json-object").unwrap(),
            ObjectType::Image,
        );
        let json = serde_json::to_value(object).unwrap();

        assert_eq!(
            json.get("cacheVersion").and_then(serde_json::Value::as_u64),
            Some(0)
        );
        assert!(json.get("cache_version").is_none());
    }
}
