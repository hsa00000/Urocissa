use anyhow::{Context, Result};
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use log::info;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

// For old redb
use redb_old::ReadableTable;

// 引入新版資料庫 Schema (假設專案結構如 new.txt 所述)
use crate::database::ops::tree::TREE;
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
// Old Data Structures (Snapshot from old.txt / previous version)
// ==================================================================================

mod old_structure {
    use super::*;
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
const NEW_DB_PATH: &str = "./db/gallery.redb";
const USER_DEFINED_DESCRIPTION: &str = "_user_defined_description";

static FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b").unwrap()
});

/// 計算 Timestamp (邏輯與舊版 generate_exif 類似)
fn compute_timestamp(db: &OldDatabase) -> i64 {
    let now_time = chrono::Local::now().naive_local();
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
                                return local_dt.timestamp_millis();
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
                        if let Some(caps) =
                            FILE_NAME_TIME_REGEX.captures(file_name.to_str().unwrap())
                        {
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
                        .timestamp_millis();
                }
            }
            "scan_time" => {
                if let Some(t) = db.alias.iter().map(|a| a.scan_time).max() {
                    return t as i64;
                }
            }
            "modified" => {
                if let Some(max_alias) = db.alias.iter().max_by_key(|a| a.scan_time) {
                    return max_alias.modified as i64;
                }
            }
            _ => {}
        }
    }
    0
}

pub fn migrate() -> Result<()> {
    // 1. 檢查是否需要遷移
    if !Path::new(OLD_DB_PATH).exists() {
        return Ok(());
    }

    println!("========================================================");
    println!(" DETECTED OLD DATABASE (v2.6.0) at {}", OLD_DB_PATH);
    println!(" A MIGRATION IS REQUIRED TO UPGRADE TO VERSION 0.18.3+");
    println!("========================================================");
    println!(" Please ensure you have BACKED UP your './db' folder.");
    println!(" The migration will read from the old DB and create a new one.");
    println!(
        " Existing data in '{}' might be overwritten/merged.",
        NEW_DB_PATH
    );
    println!("Type 'yes' to start migration:");

    let mut input = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;

    if input.trim() != "yes" {
        println!("Migration cancelled. Exiting.");
        std::process::exit(0);
    }

    info!("Starting migration...");

    // 2. 開啟舊資料庫 (唯讀，使用 redb 2.6)
    let old_db = redb_old::Database::open(OLD_DB_PATH)
        .context("Failed to open old database. Is it corrupted?")?;
    let read_txn = old_db.begin_read()?;

    // 定義舊表
    let old_data_table_def: redb_old::TableDefinition<&str, Vec<u8>> =
        redb_old::TableDefinition::new("database");
    let old_album_table_def: redb_old::TableDefinition<&str, Vec<u8>> =
        redb_old::TableDefinition::new("album");

    let old_data_table = read_txn.open_table(old_data_table_def)?;
    let old_album_table = read_txn.open_table(old_album_table_def)?;

    // 3. 開啟新資料庫 (寫入，使用全域 TREE / redb 3.1)
    let write_txn = TREE.in_disk.begin_write()?;

    {
        // --- 遷移資料 (Images/Videos) ---
        let mut object_table = write_txn.open_table(OBJECT_TABLE)?;
        let mut image_table = write_txn.open_table(META_IMAGE_TABLE)?;
        let mut video_table = write_txn.open_table(META_VIDEO_TABLE)?;
        let mut alias_table = write_txn.open_table(DATABASE_ALIAS_TABLE)?;
        let mut exif_table = write_txn.open_table(DATABASE_EXIF_TABLE)?;

        let mut tag_table =
            write_txn.open_table(crate::database::schema::relations::tag::TAG_DATABASE_TABLE)?;
        let mut album_items_table = write_txn
            .open_table(crate::database::schema::relations::album_data::ALBUM_ITEMS_TABLE)?;
        let mut item_albums_table = write_txn
            .open_table(crate::database::schema::relations::album_data::ITEM_ALBUMS_TABLE)?;

        let mut processed_count = 0;

        for result in old_data_table.iter()? {
            let (_, value) = result?;
            let old_data: OldDatabase = bitcode::decode(&value.value())?;
            let hash_str = old_data.hash.as_str();

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
            let created_time = compute_timestamp(&old_data);
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

            // 5. Insert Object
            object_table.insert(hash_str, bitcode::encode(&new_object).as_slice())?;

            // 6. Insert Metadata
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
                    image_table.insert(hash_str, bitcode::encode(&meta).as_slice())?;
                }
                ObjectType::Video => {
                    // FIX: 從 exif_vec 中讀取 duration
                    let duration = old_data
                        .exif_vec
                        .get("duration")
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);

                    let meta = VideoMetadataSchema {
                        id: old_data.hash,
                        size: old_data.size,
                        width: old_data.width,
                        height: old_data.height,
                        ext: old_data.ext,
                        duration,
                    };
                    video_table.insert(hash_str, bitcode::encode(&meta).as_slice())?;
                }
                _ => {}
            }

            // 7. Relations
            for tag in tags {
                tag_table.insert((hash_str, tag.as_str()), &())?;
            }

            for album_id in old_data.album {
                album_items_table.insert((album_id.as_str(), hash_str), &())?;
                item_albums_table.insert((hash_str, album_id.as_str()), &())?;
            }

            for alias in old_data.alias {
                let new_alias = DatabaseAliasSchema {
                    hash: hash_str.to_string(),
                    file: alias.file,
                    modified: alias.modified as i64,
                    scan_time: alias.scan_time as i64,
                };
                alias_table.insert(
                    (hash_str, new_alias.scan_time),
                    bitcode::encode(&new_alias).as_slice(),
                )?;
            }

            for (k, v) in old_data.exif_vec {
                if k == USER_DEFINED_DESCRIPTION {
                    continue;
                }
                exif_table.insert((hash_str, k.as_str()), v.as_str())?;
            }

            processed_count += 1;
            if processed_count % 1000 == 0 {
                info!("Migrated {} items...", processed_count);
            }
        }

        info!(
            "Finished migrating {} items. Starting albums...",
            processed_count
        );

        // --- Migrate ALBUMS ---
        let mut meta_album_table = write_txn.open_table(META_ALBUM_TABLE)?;
        let mut album_share_table = write_txn.open_table(ALBUM_SHARE_TABLE)?;

        for result in old_album_table.iter()? {
            let (_, value) = result?;
            let old_album: OldAlbum = bitcode::decode(&value.value())?;
            let id_str = old_album.id.as_str();

            // 1. Description
            let description = old_album
                .user_defined_metadata
                .get(USER_DEFINED_DESCRIPTION)
                .and_then(|v| v.first())
                .cloned();

            // 2. Tags
            let mut tags = old_album.tag.clone();
            let is_favorite = tags.remove("_favorite");
            let is_archived = tags.remove("_archived");
            let is_trashed = tags.remove("_trashed");

            // 3. New Object (Album)
            let new_object = ObjectSchema {
                id: old_album.id,
                created_time: old_album.created_time as i64,
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

            // 4. Meta Album
            let new_meta = AlbumMetadataSchema {
                id: old_album.id,
                title: old_album.title,
                start_time: old_album.start_time.map(|t| t as i64),
                end_time: old_album.end_time.map(|t| t as i64),
                last_modified_time: old_album.last_modified_time as i64,
                cover: old_album.cover,
                user_defined_metadata: old_album.user_defined_metadata,
                item_count: old_album.item_count,
                item_size: old_album.item_size,
            };
            meta_album_table.insert(id_str, bitcode::encode(&new_meta).as_slice())?;

            // 5. Relations
            for tag in tags {
                tag_table.insert((id_str, tag.as_str()), &())?;
            }

            // 6. Shares
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
        }
    }

    write_txn.commit()?;

    info!("Migration completed successfully.");

    // Rename old DB
    let backup_path = format!("{}.bak", OLD_DB_PATH);
    std::fs::rename(OLD_DB_PATH, &backup_path)
        .context(format!("Failed to rename old DB to {}", backup_path))?;

    info!("Old database renamed to {}", backup_path);

    Ok(())
}
