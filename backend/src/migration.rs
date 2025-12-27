//! Database Migration Module
//!
//! Handles the migration from redb 2.6.x (Old Schema) to redb 3.1.x (New AbstractData Schema).

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Write};
use std::path::Path;

use redb_old::{ReadableTable as OldReadableTable, ReadableTableMetadata};

use crate::public::constant::redb::DATA_TABLE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::{
    combined::AlbumCombined, metadata::AlbumMetadata, share::Share as NewShare,
};
use crate::public::structure::common::FileModify;
use crate::public::structure::image::{combined::ImageCombined, metadata::ImageMetadata};
use crate::public::structure::object::{ObjectSchema, ObjectType};
use crate::public::structure::video::{combined::VideoCombined, metadata::VideoMetadata};

// ==================================================================================
// Old Data Structures (Snapshot for redb 2.6.x)
// ==================================================================================

mod old_structure {
    use super::*;
    use redb_old::{TypeName, Value};

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
        pub alias: Vec<OldFileModify>,
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
            bitcode::decode(data).expect("Corrupt Data: Failed to decode OldDatabase via bitcode")
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
    pub struct OldFileModify {
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
        pub share_list: HashMap<ArrayString<64>, OldShare>,
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
            bitcode::decode(data).expect("Corrupt Data: Failed to decode OldAlbum via bitcode")
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
    pub struct OldShare {
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
const NEW_DB_PATH: &str = "./db/index_new.redb";
const BATCH_SIZE: usize = 5000;
const USER_DEFINED_DESCRIPTION: &str = "_user_defined_description";

fn transform_database_to_abstract_data(old_data: OldDatabase) -> AbstractData {
    let description = old_data.exif_vec.get(USER_DEFINED_DESCRIPTION).cloned();

    // Business Logic:
    // Legacy tags starting with "_" (e.g., _favorite) were used as pseudo-flags.
    // We convert them to explicit boolean fields and remove them from the generic tag list.
    let mut tags = old_data.tag.clone();
    let is_favorite = tags.remove("_favorite");
    let is_archived = tags.remove("_archived");
    let is_trashed = tags.remove("_trashed");

    let alias: Vec<FileModify> = old_data
        .alias
        .iter()
        .map(|a| FileModify {
            file: a.file.clone(),
            modified: a.modified,
            scan_time: a.scan_time,
        })
        .collect();

    let mut exif_vec = old_data.exif_vec.clone();
    exif_vec.remove(USER_DEFINED_DESCRIPTION);

    let thumbhash = if old_data.thumbhash.is_empty() {
        None
    } else {
        Some(old_data.thumbhash)
    };

    if old_data.ext_type == "video" {
        let duration = old_data
            .exif_vec
            .get("duration")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let object = ObjectSchema {
            id: old_data.hash,
            obj_type: ObjectType::Video,
            pending: old_data.pending,
            thumbhash,
            description,
            tags,
            is_favorite,
            is_archived,
            is_trashed,
        };

        let metadata = VideoMetadata {
            id: old_data.hash,
            size: old_data.size,
            width: old_data.width,
            height: old_data.height,
            ext: old_data.ext,
            duration,
            albums: old_data.album,
            exif_vec,
            alias,
        };

        AbstractData::Video(VideoCombined { object, metadata })
    } else {
        let phash = if old_data.phash.is_empty() {
            None
        } else {
            Some(old_data.phash)
        };

        let object = ObjectSchema {
            id: old_data.hash,
            obj_type: ObjectType::Image,
            pending: old_data.pending,
            thumbhash,
            description,
            tags,
            is_favorite,
            is_archived,
            is_trashed,
        };

        let metadata = ImageMetadata {
            id: old_data.hash,
            size: old_data.size,
            width: old_data.width,
            height: old_data.height,
            ext: old_data.ext,
            phash,
            albums: old_data.album,
            exif_vec,
            alias,
        };

        AbstractData::Image(ImageCombined { object, metadata })
    }
}

fn transform_album_to_abstract_data(old_album: OldAlbum) -> AbstractData {
    let description = old_album
        .user_defined_metadata
        .get(USER_DEFINED_DESCRIPTION)
        .and_then(|v| v.first())
        .cloned();

    // Business Logic: Convert legacy tag-flags to boolean fields
    let mut tags = old_album.tag.clone();
    let is_favorite = tags.remove("_favorite");
    let is_archived = tags.remove("_archived");
    let is_trashed = tags.remove("_trashed");

    let share_list: HashMap<ArrayString<64>, NewShare> = old_album
        .share_list
        .into_iter()
        .map(|(key, old_share)| {
            (
                key,
                NewShare {
                    url: old_share.url,
                    description: old_share.description,
                    password: old_share.password,
                    show_metadata: old_share.show_metadata,
                    show_download: old_share.show_download,
                    show_upload: old_share.show_upload,
                    exp: old_share.exp,
                },
            )
        })
        .collect();

    let object = ObjectSchema {
        id: old_album.id,
        obj_type: ObjectType::Album,
        pending: old_album.pending,
        thumbhash: old_album.thumbhash,
        description,
        tags,
        is_favorite,
        is_archived,
        is_trashed,
    };

    let metadata = AlbumMetadata {
        id: old_album.id,
        title: old_album.title,
        created_time: old_album.created_time as i64,
        start_time: old_album.start_time.map(|t| t as i64),
        end_time: old_album.end_time.map(|t| t as i64),
        last_modified_time: old_album.last_modified_time as i64,
        cover: old_album.cover,
        item_count: old_album.item_count,
        item_size: old_album.item_size,
        share_list,
    };

    AbstractData::Album(AlbumCombined { object, metadata })
}

/// Checks if a migration is required.
///
/// Strategy:
/// 1. Try opening with the NEW driver (v3.1). If successful, no migration needed.
/// 2. If step 1 fails, try opening with the OLD driver (v2.6) to confirm it is a valid legacy DB.
fn needs_migration() -> bool {
    if !Path::new(OLD_DB_PATH).exists() {
        return false;
    }

    if redb::Database::open(OLD_DB_PATH).is_ok() {
        println!("[INFO] DB is compatible with current driver. No migration needed.");
        return false;
    }

    // Defensive: Use catch_unwind as a safety net against potential FFI/panic issues
    // when the old driver encounters a corrupted or unrecognized file format.
    let result = std::panic::catch_unwind(|| redb_old::Database::open(OLD_DB_PATH));

    match result {
        Ok(Ok(db)) => {
            if let Ok(txn) = db.begin_read() {
                // Check for existence of legacy tables
                let has_db_table = txn
                    .open_table(redb_old::TableDefinition::<&str, OldDatabase>::new(
                        "database",
                    ))
                    .is_ok();
                let has_album_table = txn
                    .open_table(redb_old::TableDefinition::<&str, OldAlbum>::new("album"))
                    .is_ok();

                has_db_table || has_album_table
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Executes the migration from redb 2.6 to 3.1.
///
/// This process involves:
/// 1. Verifying migration necessity.
/// 2. Creating a backup of the old database.
/// 3. Transforming all data records to the new `AbstractData` format.
pub fn migrate() -> Result<()> {
    if !needs_migration() {
        return Ok(());
    }

    println!("========================================================");
    println!(" DETECTED OLD DATABASE (redb 2.6.x) at {}", OLD_DB_PATH);
    println!(" A MIGRATION IS REQUIRED TO UPGRADE TO VERSION 0.19+");
    println!("========================================================");

    println!(" Please ensure you have BACKED UP your './db' folder.");
    println!(" The migration will read from the old DB and create a new one.");
    println!("Type 'yes' to start migration:");

    let mut input = String::new();
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;

    if input.trim() != "yes" {
        println!("Migration cancelled.");
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

    let new_db =
        redb::Database::create(NEW_DB_PATH).context("Failed to create new database file.")?;
    let write_txn = new_db.begin_write()?;

    // Migrating DATA (Images/Videos)
    {
        println!("Migrating Data...");
        let mut data_table = write_txn.open_table(DATA_TABLE)?;

        let total_items = old_data_table.len()?;
        println!("Found {} items to migrate.", total_items);

        let mut processed_count = 0;
        let mut batch_buffer: Vec<OldDatabase> = Vec::with_capacity(BATCH_SIZE);

        let mut commit_batch = |batch: Vec<OldDatabase>| -> Result<()> {
            // Performance: Use parallel iterator as struct transformation and bitcode decoding
            // are CPU-bound operations.
            let transformed_batch: Vec<AbstractData> = batch
                .into_par_iter()
                .map(transform_database_to_abstract_data)
                .collect();

            for abstract_data in transformed_batch {
                data_table.insert(abstract_data.hash().as_str(), &abstract_data)?;
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

    // Migrating ALBUMS
    {
        println!("Migrating Albums...");
        let mut data_table = write_txn.open_table(DATA_TABLE)?;

        let total_albums = old_album_table.len()?;
        println!("Found {} albums to migrate.", total_albums);

        let mut processed_count = 0;

        for result in old_album_table.iter()? {
            let (_, value) = result?;
            let abstract_data = transform_album_to_abstract_data(value.value());
            data_table.insert(abstract_data.hash().as_str(), &abstract_data)?;

            processed_count += 1;
            if processed_count % 100 == 0 {
                println!("Migrated {} / {} albums...", processed_count, total_albums);
            }
        }

        println!("Album migration completed. Total: {}", processed_count);
    }

    write_txn.commit()?;

    println!("Migration completed successfully.");

    // Explicitly drop handles to release file locks before renaming
    drop(read_txn);
    drop(old_db);

    let backup_path = format!("{}.bak", OLD_DB_PATH);
    std::fs::rename(OLD_DB_PATH, &backup_path)
        .context(format!("Failed to rename old DB to {}", backup_path))?;
    println!("Old database renamed to {}", backup_path);

    std::fs::rename(NEW_DB_PATH, OLD_DB_PATH)
        .context("Failed to rename new DB to original path")?;
    println!("New database moved to {}", OLD_DB_PATH);

    println!("SUCCESS: Migration completed. Backup at {}", backup_path);
    Ok(())
}
