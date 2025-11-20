use crate::operations::utils::timestamp::get_current_timestamp_u64;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Album;
use crate::public::structure::database_struct::database::definition::Database;
use crate::public::structure::database_struct::database_timestamp::DatabaseTimestamp;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_expire::UpdateExpireTask;
use arrayvec::ArrayString;
use mini_executor::BatchTask;
use rayon::prelude::*;
use rusqlite::Connection;
use serde_json;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;
use std::time::Instant;

static ALLOWED_KEYS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "Make",
        "Model",
        "FNumber",
        "ExposureTime",
        "FocalLength",
        "PhotographicSensitivity",
        "DateTimeOriginal",
        "duration",
        "rotation",
    ]
    .iter()
    .cloned()
    .collect()
});

pub struct UpdateTreeTask;

impl BatchTask for UpdateTreeTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            update_tree_task();
        }
    }
}

fn update_tree_task() {
    let start_time = Instant::now();
    let conn = Connection::open("gallery.db").expect("Failed to open database");

    let priority_list = vec!["DateTimeOriginal", "filename", "modified", "scan_time"];

    let mut database_timestamp_vec: Vec<DatabaseTimestamp> = {
        let mut stmt = conn.prepare("SELECT * FROM database").unwrap();
        let rows: Vec<Database> = stmt
            .query_map([], |row| {
                let hash: String = row.get("hash")?;
                let size: u64 = row.get("size")?;
                let width: u32 = row.get("width")?;
                let height: u32 = row.get("height")?;
                let thumbhash: Vec<u8> = row.get("thumbhash")?;
                let phash: Vec<u8> = row.get("phash")?;
                let ext: String = row.get("ext")?;
                let exif_vec_str: String = row.get("exif_vec")?;
                let exif_vec: BTreeMap<String, String> =
                    serde_json::from_str(&exif_vec_str).unwrap_or_default();
                let tag_str: String = row.get("tag")?;
                let tag: HashSet<String> = serde_json::from_str(&tag_str).unwrap_or_default();
                let album_str: String = row.get("album")?;
                let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap_or_default();
                let album: HashSet<ArrayString<64>> = album_vec
                    .into_iter()
                    .filter_map(|s| ArrayString::from(&s).ok())
                    .collect();
                let alias_str: String = row.get("alias")?;
                let alias: Vec<crate::public::structure::database_struct::file_modify::FileModify> =
                    serde_json::from_str(&alias_str).unwrap_or_default();
                let ext_type: String = row.get("ext_type")?;
                let pending: bool = row.get::<_, i32>("pending")? != 0;
                Ok(Database {
                    hash: ArrayString::from(&hash).unwrap(),
                    size,
                    width,
                    height,
                    thumbhash,
                    phash,
                    ext,
                    exif_vec,
                    tag,
                    album,
                    alias,
                    ext_type,
                    pending,
                })
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        rows.into_par_iter()
            .map(|mut db| {
                db.exif_vec
                    .retain(|k, _| ALLOWED_KEYS.contains(&k.as_str()));
                DatabaseTimestamp::new(AbstractData::Database(db), &priority_list)
            })
            .collect()
    };

    let album_vec: Vec<DatabaseTimestamp> = {
        let mut stmt = conn.prepare("SELECT * FROM album").unwrap();
        let rows: Vec<Album> = stmt
            .query_map([], |row| {
                let id: String = row.get("id")?;
                let title: Option<String> = row.get("title")?;
                let created_time: u128 = row.get::<_, i64>("created_time")? as u128;
                let start_time: Option<u128> =
                    row.get::<_, Option<i64>>("start_time")?.map(|t| t as u128);
                let end_time: Option<u128> =
                    row.get::<_, Option<i64>>("end_time")?.map(|t| t as u128);
                let last_modified_time: u128 = row.get::<_, i64>("last_modified_time")? as u128;
                let cover_str: Option<String> = row.get("cover")?;
                let cover: Option<ArrayString<64>> =
                    cover_str.and_then(|s| ArrayString::from(&s).ok());
                let thumbhash: Option<Vec<u8>> = row.get("thumbhash")?;
                let user_defined_metadata_str: String = row.get("user_defined_metadata")?;
                let user_defined_metadata: HashMap<String, Vec<String>> =
                    serde_json::from_str(&user_defined_metadata_str).unwrap_or_default();
                let share_list_str: String = row.get("share_list")?;
                let share_list: HashMap<ArrayString<64>, crate::public::structure::album::Share> =
                    serde_json::from_str(&share_list_str).unwrap_or_default();
                let tag_str: String = row.get("tag")?;
                let tag: HashSet<String> = serde_json::from_str(&tag_str).unwrap_or_default();
                let width: u32 = row.get("width")?;
                let height: u32 = row.get("height")?;
                let item_count: usize = row.get::<_, i64>("item_count")? as usize;
                let item_size: u64 = row.get("item_size")?;
                let pending: bool = row.get::<_, i32>("pending")? != 0;
                Ok(Album {
                    id: ArrayString::from(&id).unwrap(),
                    title,
                    created_time,
                    start_time,
                    end_time,
                    last_modified_time,
                    cover,
                    thumbhash,
                    user_defined_metadata,
                    share_list,
                    tag,
                    width,
                    height,
                    item_count,
                    item_size,
                    pending,
                })
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        rows.into_par_iter()
            .map(|album| DatabaseTimestamp::new(AbstractData::Album(album), &priority_list))
            .collect()
    };

    database_timestamp_vec.extend(album_vec);
    database_timestamp_vec.par_sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    *TREE.in_memory.write().unwrap() = database_timestamp_vec;

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = get_current_timestamp_u64();
    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "In-memory cache updated ({}).", current_timestamp);
}
