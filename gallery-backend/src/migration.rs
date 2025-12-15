use anyhow::{Context, Result};
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

// 引入舊版 redb 的 ReadableTable trait
use redb_old::{ReadableTable as OldReadableTable, ReadableTableMetadata};

// Import New Schema
use crate::database::ops::tree::TREE;
use crate::database::schema::relations::album_data::{ALBUM_ITEMS_TABLE, ITEM_ALBUMS_TABLE};
use crate::database::schema::relations::tag::OBJECT_TAGS_TABLE;
use crate::database::schema::{
    meta_album::{AlbumMetadataSchema, META_ALBUM_TABLE},
    meta_image::{ImageMetadataSchema, META_IMAGE_TABLE},
    meta_video::{META_VIDEO_TABLE, VideoMetadataSchema},
    object::{OBJECT_TABLE, ObjectSchema, ObjectType},
    relations::{
        album_share::{ALBUM_SHARE_TABLE, Share as NewShare},
        alias::{DATABASE_ALIAS_TABLE, DatabaseAliasSchema},
        exif::DATABASE_EXIF_TABLE,
    },
};

// ==================================================================================
// Old Data Structures
// ==================================================================================

mod old_structure {
    use super::*;
    use redb_old::{TypeName, Value};
    use std::collections::HashMap;

    #[derive(Debug, Clone, Deserialize, Default, Serialize, Decode, Encode, PartialEq, Eq)]
    pub struct Database {
        pub hash: ArrayString<64>,
        pub size: u64,
        pub width: u32,
        pub height: u32,
        pub thumbhash: Vec<u8>,
        pub phash: Vec<u8>,
        pub ext: String,
        pub exif_vec: BTreeMap<String, String>,
        pub tag: HashSet<String>,
        pub album: HashSet<ArrayString<64>>,
        pub alias: Vec<FileModify>,
        pub ext_type: String,
        pub pending: bool,
    }

    impl Value for Database {
        type SelfType<'a> = Self;
        type AsBytes<'a> = Vec<u8>;

        fn fixed_width() -> Option<usize> {
            None
        }

        fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
        where
            Self: 'a,
        {
            bitcode::decode(data).expect("Failed to decode OldDatabase")
        }

        fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
            bitcode::encode(value)
        }

        fn type_name() -> TypeName {
            TypeName::new("Database")
        }
    }

    #[derive(
        Debug,
        Default,
        Clone,
        Deserialize,
        Serialize,
        Decode,
        Encode,
        Hash,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
    )]
    #[serde(rename_all = "camelCase")]
    pub struct FileModify {
        pub file: String,
        pub modified: u128,
        pub scan_time: u128,
    }

    #[derive(Debug, Clone, Deserialize, Default, Serialize, Decode, Encode, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct Album {
        pub id: ArrayString<64>,
        pub title: Option<String>,
        pub created_time: u128,
        pub start_time: Option<u128>,
        pub end_time: Option<u128>,
        pub last_modified_time: u128,
        pub cover: Option<ArrayString<64>>,
        pub thumbhash: Option<Vec<u8>>,
        pub user_defined_metadata: HashMap<String, Vec<String>>,
        pub share_list: HashMap<ArrayString<64>, Share>,
        pub tag: HashSet<String>,
        pub width: u32,
        pub height: u32,
        pub item_count: usize,
        pub item_size: u64,
        pub pending: bool,
    }

    impl Value for Album {
        type SelfType<'a> = Self;
        type AsBytes<'a> = Vec<u8>;

        fn fixed_width() -> Option<usize> {
            None
        }

        fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
        where
            Self: 'a,
        {
            bitcode::decode(data).expect("Failed to decode OldAlbum")
        }

        fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
            bitcode::encode(value)
        }

        fn type_name() -> TypeName {
            TypeName::new("Album")
        }
    }

    #[derive(
        Debug, Clone, Deserialize, Default, Serialize, Decode, Encode, PartialEq, Eq, Hash,
    )]
    #[serde(rename_all = "camelCase")]
    pub struct Share {
        pub url: ArrayString<64>,
        pub description: String,
        pub password: Option<String>,
        pub show_metadata: bool,
        pub show_download: bool,
        pub show_upload: bool,
        pub exp: u64,
    }
}

use old_structure::{Album as OldAlbum, Database as OldDatabase};

// ==================================================================================
// Migration Logic
// ==================================================================================

const OLD_DB_PATH: &str = "./db/index.redb";
const BATCH_SIZE: usize = 5000;
const USER_DEFINED_DESCRIPTION: &str = "_user_defined_description";

struct TransformedData {
    object: (String, Vec<u8>),
    meta_image: Option<(String, Vec<u8>)>,
    meta_video: Option<(String, Vec<u8>)>,
    tags: Vec<(String, String)>,
    album_items: Vec<(String, String)>,
    item_albums: Vec<(String, String)>,
    aliases: Vec<((String, i64), Vec<u8>)>,
    exifs: Vec<((String, String), String)>,
}

static FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b").unwrap()
});

fn compute_timestamp(db: &OldDatabase, now_time: NaiveDateTime) -> i64 {
    let priority_list = &["DateTimeOriginal", "filename", "modified", "scan_time"];

    for &field in priority_list {
        match field {
            "DateTimeOriginal" => {
                if let Some(value) = db.exif_vec.get("DateTimeOriginal") {
                    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                    {
                        if let Some(local_dt) =
                            chrono::Local.from_local_datetime(&naive_dt).single()
                        {
                            if local_dt.naive_local() <= now_time {
                                return local_dt.timestamp();
                            }
                        }
                    }
                }
            }
            "filename" => {
                let mut max_time: Option<NaiveDateTime> = None;
                for alias in &db.alias {
                    let path = PathBuf::from(&alias.file);
                    if let Some(file_name) = path.file_name() {
                        let file_name_str = file_name.to_string_lossy();
                        if let Some(caps) = FILE_NAME_TIME_REGEX.captures(&file_name_str) {
                            let parts: Option<(i32, u32, u32, u32, u32, u32)> = (|| {
                                Some((
                                    caps[1].parse().ok()?,
                                    caps[2].parse().ok()?,
                                    caps[3].parse().ok()?,
                                    caps[4].parse().ok()?,
                                    caps[5].parse().ok()?,
                                    caps[6].parse().ok()?,
                                ))
                            })(
                            );

                            if let Some((year, month, day, hour, minute, second)) = parts {
                                if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                                    if let Some(time) =
                                        NaiveTime::from_hms_opt(hour, minute, second)
                                    {
                                        let datetime = NaiveDateTime::new(date, time);
                                        if datetime <= now_time {
                                            max_time = Some(
                                                max_time.map_or(datetime, |t| t.max(datetime)),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(datetime) = max_time {
                    return chrono::Local
                        .from_local_datetime(&datetime)
                        .unwrap()
                        .timestamp();
                }
            }
            "scan_time" => {
                if let Some(t) = db.alias.iter().map(|a| a.scan_time).max() {
                    return (t / 1000) as i64;
                }
            }
            "modified" => {
                if let Some(max_alias) = db.alias.iter().max_by_key(|a| a.scan_time) {
                    return (max_alias.modified / 1000) as i64;
                }
            }
            _ => {}
        }
    }
    0
}

pub fn migrate() -> Result<()> {
    if !Path::new(OLD_DB_PATH).exists() {
        return Ok(());
    }

    println!("========================================================");
    println!(" DETECTED OLD DATABASE (v2.6.0) at {}", OLD_DB_PATH);
    println!(" A MIGRATION IS REQUIRED TO UPGRADE TO VERSION 0.18.3+");
    println!("========================================================");
    println!(" Please ensure you have BACKED UP your './db' folder.");
    println!(" The migration will read from the old DB and create a new one.");
    println!("Type 'yes' to start migration:");

    let mut input = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;

    if input.trim() != "yes" {
        println!("Migration cancelled. Exiting.");
        std::process::exit(0);
    }

    println!("Starting migration...");

    let old_db =
        redb_old::Database::open(OLD_DB_PATH).context("Failed to open old database file.")?;
    let read_txn = old_db.begin_read()?;

    let old_data_table = read_txn.open_table(
        redb_old::TableDefinition::<&str, OldDatabase>::new("database"),
    )?;
    let old_album_table =
        read_txn.open_table(redb_old::TableDefinition::<&str, OldAlbum>::new("album"))?;

    let write_txn = TREE.in_disk.begin_write()?;

    // ==========================================
    // 1. Migrate DATA (Images/Videos)
    // ==========================================
    {
        let mut object_table = write_txn.open_table(OBJECT_TABLE)?;
        let mut image_table = write_txn.open_table(META_IMAGE_TABLE)?;
        let mut video_table = write_txn.open_table(META_VIDEO_TABLE)?;
        let mut alias_table = write_txn.open_table(DATABASE_ALIAS_TABLE)?;
        let mut exif_table = write_txn.open_table(DATABASE_EXIF_TABLE)?;
        let mut tag_table = write_txn.open_table(OBJECT_TAGS_TABLE)?;
        let mut album_items_table = write_txn.open_table(ALBUM_ITEMS_TABLE)?;
        let mut item_albums_table = write_txn.open_table(ITEM_ALBUMS_TABLE)?;

        let total_items = old_data_table.len()?;
        println!("Found {} items to migrate.", total_items);

        let mut processed_count = 0;
        let mut batch_buffer: Vec<OldDatabase> = Vec::with_capacity(BATCH_SIZE);

        let now_time = chrono::Local::now().naive_local();

        let mut commit_batch = |batch: Vec<OldDatabase>| -> Result<()> {
            let transformed_batch: Vec<TransformedData> = batch
                .into_par_iter()
                .map(|old_data| {
                    let hash_str = old_data.hash.as_str().to_string();

                    // 1. Description
                    let description = old_data.exif_vec.get(USER_DEFINED_DESCRIPTION).cloned();

                    // 2. Tags
                    let mut tags = old_data.tag.clone();
                    let is_favorite = tags.remove("_favorite");
                    let is_archived = tags.remove("_archived");
                    let is_trashed = tags.remove("_trashed");

                    // 3. ObjectType
                    let obj_type = if old_data.ext_type == "video" {
                        ObjectType::Video
                    } else {
                        ObjectType::Image
                    };

                    // 4. Create New Object

                    let created_time = compute_timestamp(&old_data, now_time);
                    let new_object = ObjectSchema {
                        id: old_data.hash,
                        created_time,
                        obj_type,
                        thumbhash: if old_data.thumbhash.is_empty() {
                            None
                        } else {
                            Some(old_data.thumbhash)
                        },
                        pending: old_data.pending,
                        description,
                        tags: tags.clone(),
                        is_favorite,
                        is_archived,
                        is_trashed,
                    };
                    let object_bytes = bitcode::encode(&new_object);

                    // 5. Metadata
                    let mut meta_image = None;
                    let mut meta_video = None;

                    match obj_type {
                        ObjectType::Image => {
                            let meta = ImageMetadataSchema {
                                id: old_data.hash,
                                size: old_data.size,
                                width: old_data.width,
                                height: old_data.height,
                                ext: old_data.ext,
                                phash: if old_data.phash.is_empty() {
                                    None
                                } else {
                                    Some(old_data.phash)
                                },
                            };
                            meta_image = Some((hash_str.clone(), bitcode::encode(&meta)));
                        }
                        ObjectType::Video => {
                            let duration = old_data
                                .exif_vec
                                .get("duration")
                                .and_then(|s| s.parse::<f32>().ok())
                                .unwrap_or(0.0);
                            let meta = VideoMetadataSchema {
                                id: old_data.hash,
                                size: old_data.size,
                                width: old_data.width,
                                height: old_data.height,
                                ext: old_data.ext,
                                duration: duration as f64,
                            };
                            meta_video = Some((hash_str.clone(), bitcode::encode(&meta)));
                        }
                        _ => {}
                    }

                    // 6. Pre-calculate Relations
                    let mut tag_rels = Vec::new();
                    for tag in tags {
                        tag_rels.push((hash_str.clone(), tag));
                    }

                    let mut alb_item_rels = Vec::new();
                    let mut item_alb_rels = Vec::new();
                    for album_id in old_data.album {
                        alb_item_rels.push((album_id.to_string(), hash_str.clone()));
                        item_alb_rels.push((hash_str.clone(), album_id.to_string()));
                    }

                    let mut alias_rels = Vec::new();
                    for alias in old_data.alias {
                        // 將毫秒轉為秒
                        let scan_time_sec = (alias.scan_time / 1000) as i64;
                        let modified_sec = (alias.modified / 1000) as i64;

                        let new_alias = DatabaseAliasSchema {
                            hash: hash_str.clone(),
                            file: alias.file,
                            modified: modified_sec,
                            scan_time: scan_time_sec,
                        };
                        alias_rels.push((
                            (hash_str.clone(), new_alias.scan_time),
                            bitcode::encode(&new_alias),
                        ));
                    }

                    let mut exif_rels = Vec::new();
                    for (k, v) in old_data.exif_vec {
                        if k != USER_DEFINED_DESCRIPTION {
                            exif_rels.push(((hash_str.clone(), k), v));
                        }
                    }

                    TransformedData {
                        object: (hash_str, object_bytes),
                        meta_image,
                        meta_video,
                        tags: tag_rels,
                        album_items: alb_item_rels,
                        item_albums: item_alb_rels,
                        aliases: alias_rels,
                        exifs: exif_rels,
                    }
                })
                .collect();

            // 寫入數據
            for item in transformed_batch {
                object_table.insert(item.object.0.as_str(), item.object.1.as_slice())?;

                if let Some((k, v)) = item.meta_image {
                    image_table.insert(k.as_str(), v.as_slice())?;
                }
                if let Some((k, v)) = item.meta_video {
                    video_table.insert(k.as_str(), v.as_slice())?;
                }
                for (k_hash, k_tag) in item.tags {
                    tag_table.insert((k_hash.as_str(), k_tag.as_str()), ())?;
                }
                for (k_alb, k_item) in item.album_items {
                    album_items_table.insert((k_alb.as_str(), k_item.as_str()), ())?;
                }
                for (k_item, k_alb) in item.item_albums {
                    item_albums_table.insert((k_item.as_str(), k_alb.as_str()), ())?;
                }
                for (key, val) in item.aliases {
                    alias_table.insert((key.0.as_str(), key.1), val.as_slice())?;
                }
                for (key, val) in item.exifs {
                    exif_table.insert((key.0.as_str(), key.1.as_str()), val.as_str())?;
                }
            }
            Ok(())
        };

        for result in old_data_table.iter()? {
            let (_, value) = result?;
            batch_buffer.push(value.value());

            if batch_buffer.len() >= BATCH_SIZE {
                commit_batch(std::mem::take(&mut batch_buffer))?;
                processed_count += BATCH_SIZE;
                println!("Migrated {} / {} items...", processed_count, total_items);
            }
        }
        if !batch_buffer.is_empty() {
            let len = batch_buffer.len();
            commit_batch(batch_buffer)?;
            processed_count += len;
        }
        println!("Data migration completed. Total: {}", processed_count);
    }

    // ==========================================
    // 2. Migrate ALBUMS
    // ==========================================
    {
        let mut object_table = write_txn.open_table(OBJECT_TABLE)?;
        let mut meta_album_table = write_txn.open_table(META_ALBUM_TABLE)?;
        let mut album_share_table = write_txn.open_table(ALBUM_SHARE_TABLE)?;
        let mut tag_table = write_txn.open_table(OBJECT_TAGS_TABLE)?;

        let total_albums = old_album_table.len()?;
        println!("Found {} albums to migrate.", total_albums);

        let mut processed_count = 0;

        for result in old_album_table.iter()? {
            let (_, value) = result?;
            let old_album: OldAlbum = value.value();
            let id_str = old_album.id.as_str();

            let description = old_album
                .user_defined_metadata
                .get(USER_DEFINED_DESCRIPTION)
                .and_then(|v| v.first())
                .cloned();

            let mut tags = old_album.tag.clone();
            let is_favorite = tags.remove("_favorite");
            let is_archived = tags.remove("_archived");
            let is_trashed = tags.remove("_trashed");

            // 時間轉換：毫秒 -> 秒
            let created_time = (old_album.created_time / 1000) as i64;

            let new_object = ObjectSchema {
                id: old_album.id,
                created_time,
                obj_type: ObjectType::Album,
                thumbhash: old_album.thumbhash,
                pending: old_album.pending,
                description,
                tags: tags.clone(),
                is_favorite,
                is_archived,
                is_trashed,
            };
            object_table.insert(id_str, bitcode::encode(&new_object).as_slice())?;

            let new_meta = AlbumMetadataSchema {
                id: old_album.id,
                title: old_album.title,
                cover: old_album.cover,
                // 時間轉換：毫秒 -> 秒
                start_time: old_album.start_time.map(|t| (t / 1000) as i64),
                end_time: old_album.end_time.map(|t| (t / 1000) as i64),
                last_modified_time: (old_album.last_modified_time / 1000) as i64,
                user_defined_metadata: old_album.user_defined_metadata.into_iter().collect(),
                item_count: old_album.item_count,
                item_size: old_album.item_size,
            };
            meta_album_table.insert(id_str, bitcode::encode(&new_meta).as_slice())?;

            for tag in tags {
                tag_table.insert((id_str, tag.as_str()), ())?;
            }

            for (share_url, old_share) in old_album.share_list {
                let new_share = NewShare {
                    url: old_share.url,
                    description: old_share.description,
                    password: old_share.password,
                    show_metadata: old_share.show_metadata,
                    show_download: old_share.show_download,
                    show_upload: old_share.show_upload,
                    exp: old_share.exp,
                };
                album_share_table.insert(
                    (id_str, share_url.as_str()),
                    bitcode::encode(&new_share).as_slice(),
                )?;
            }

            processed_count += 1;
            if processed_count % 100 == 0 {
                println!("Migrated {} / {} albums...", processed_count, total_albums);
            }
        }
        println!("Album migration completed. Total: {}", processed_count);
    }

    write_txn.commit()?;
    println!("Migration completed successfully.");

    let backup_path = format!("{}.bak", OLD_DB_PATH);
    std::fs::rename(OLD_DB_PATH, &backup_path)
        .context(format!("Failed to rename old DB to {}", backup_path))?;
    println!("Old database renamed to {}", backup_path);

    Ok(())
}
