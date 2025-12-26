//! Database Migration Module
//!
//! This module handles the migration from redb 2.6.x to redb 3.1.x.
//! It reads from the old database format and converts data to the new AbstractData format.

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Write};
use std::path::Path;

// Import old redb's ReadableTable trait
use redb_old::{ReadableTable as OldReadableTable, ReadableTableMetadata};

// Import current schema
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
// Old Data Structures (for redb 2.6.x)
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

/// Transform old Database (Image/Video) to new AbstractData
fn transform_database_to_abstract_data(old_data: OldDatabase) -> AbstractData {
    // 1. Extract description from exif_vec
    let description = old_data.exif_vec.get(USER_DEFINED_DESCRIPTION).cloned();

    // 2. Process tags - remove special tags and convert to flags
    let mut tags = old_data.tag.clone();
    let is_favorite = tags.remove("_favorite");
    let is_archived = tags.remove("_archived");
    let is_trashed = tags.remove("_trashed");

    // 3. Convert alias
    let alias: Vec<FileModify> = old_data
        .alias
        .iter()
        .map(|a| FileModify {
            file: a.file.clone(),
            modified: a.modified,
            scan_time: a.scan_time,
        })
        .collect();

    // 4. Create exif_vec without the description key
    let mut exif_vec = old_data.exif_vec.clone();
    exif_vec.remove(USER_DEFINED_DESCRIPTION);

    // 5. Determine object type and create appropriate AbstractData
    if old_data.ext_type == "video" {
        // Video
        let duration = old_data
            .exif_vec
            .get("duration")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let object = ObjectSchema {
            id: old_data.hash,
            obj_type: ObjectType::Video,
            pending: old_data.pending,
            thumbhash: if old_data.thumbhash.is_empty() {
                None
            } else {
                Some(old_data.thumbhash)
            },
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
        // Image
        let object = ObjectSchema {
            id: old_data.hash,
            obj_type: ObjectType::Image,
            pending: old_data.pending,
            thumbhash: if old_data.thumbhash.is_empty() {
                None
            } else {
                Some(old_data.thumbhash)
            },
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
            phash: if old_data.phash.is_empty() {
                None
            } else {
                Some(old_data.phash)
            },
            albums: old_data.album,
            exif_vec,
            alias,
        };

        AbstractData::Image(ImageCombined { object, metadata })
    }
}

/// Transform old Album to new AbstractData::Album
fn transform_album_to_abstract_data(old_album: OldAlbum) -> AbstractData {
    // 1. Extract description from user_defined_metadata
    let description = old_album
        .user_defined_metadata
        .get(USER_DEFINED_DESCRIPTION)
        .and_then(|v| v.first())
        .cloned();

    // 2. Process tags - remove special tags and convert to flags
    let mut tags = old_album.tag.clone();
    let is_favorite = tags.remove("_favorite");
    let is_archived = tags.remove("_archived");
    let is_trashed = tags.remove("_trashed");

    // 3. Convert share_list
    let share_list: HashMap<ArrayString<64>, NewShare> = old_album
        .share_list
        .into_iter()
        .map(|(key, old_share)| {
            let new_share = NewShare {
                url: old_share.url,
                description: old_share.description,
                password: old_share.password,
                show_metadata: old_share.show_metadata,
                show_download: old_share.show_download,
                show_upload: old_share.show_upload,
                exp: old_share.exp,
            };
            (key, new_share)
        })
        .collect();

    // 4. Create ObjectSchema
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

    // 5. Create AlbumMetadata
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

/// Check if the old database exists and needs migration
fn needs_migration() -> bool {
    if !Path::new(OLD_DB_PATH).exists() {
        return false;
    }

    // Try to open with old redb version to see if it's an old format database
    match redb_old::Database::open(OLD_DB_PATH) {
        Ok(db) => {
            // Check if it has the old table structure
            if let Ok(txn) = db.begin_read() {
                let has_old_database_table = txn
                    .open_table(redb_old::TableDefinition::<&str, OldDatabase>::new("database"))
                    .is_ok();
                let has_old_album_table = txn
                    .open_table(redb_old::TableDefinition::<&str, OldAlbum>::new("album"))
                    .is_ok();
                has_old_database_table || has_old_album_table
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Main migration function
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
        println!("Migration cancelled. Exiting.");
        std::process::exit(0);
    }

    println!("Starting migration...");

    // Open the old database
    let old_db =
        redb_old::Database::open(OLD_DB_PATH).context("Failed to open old database file.")?;
    let read_txn = old_db.begin_read()?;

    // Open old tables
    let old_data_table = read_txn.open_table(
        redb_old::TableDefinition::<&str, OldDatabase>::new("database"),
    )?;
    let old_album_table =
        read_txn.open_table(redb_old::TableDefinition::<&str, OldAlbum>::new("album"))?;

    // Create new database
    let new_db = redb::Database::create(NEW_DB_PATH)
        .context("Failed to create new database file.")?;
    let write_txn = new_db.begin_write()?;

    // ==========================================
    // 1. Migrate DATA (Images/Videos)
    // ==========================================
    {
        let mut data_table = write_txn.open_table(DATA_TABLE)?;

        let total_items = old_data_table.len()?;
        println!("Found {} items to migrate.", total_items);

        let mut processed_count = 0;
        let mut batch_buffer: Vec<OldDatabase> = Vec::with_capacity(BATCH_SIZE);

        let mut commit_batch = |batch: Vec<OldDatabase>| -> Result<()> {
            // Transform in parallel
            let transformed_batch: Vec<AbstractData> = batch
                .into_par_iter()
                .map(|old_data| transform_database_to_abstract_data(old_data))
                .collect();

            // Write sequentially
            for abstract_data in transformed_batch {
                let hash = abstract_data.hash();
                data_table.insert(hash.as_str(), &abstract_data)?;
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

        // Process remaining items
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
        let mut data_table = write_txn.open_table(DATA_TABLE)?;

        let total_albums = old_album_table.len()?;
        println!("Found {} albums to migrate.", total_albums);

        let mut processed_count = 0;

        for result in old_album_table.iter()? {
            let (_, value) = result?;
            let old_album: OldAlbum = value.value();

            let abstract_data = transform_album_to_abstract_data(old_album);
            let hash = abstract_data.hash();
            data_table.insert(hash.as_str(), &abstract_data)?;

            processed_count += 1;
            if processed_count % 100 == 0 {
                println!("Migrated {} / {} albums...", processed_count, total_albums);
            }
        }
        println!("Album migration completed. Total: {}", processed_count);
    }

    // Commit the transaction
    write_txn.commit()?;
    println!("Migration completed successfully.");

    // Close old database before renaming
    drop(read_txn);
    drop(old_db);

    // Rename old database to backup
    let backup_path = format!("{}.bak", OLD_DB_PATH);
    std::fs::rename(OLD_DB_PATH, &backup_path)
        .context(format!("Failed to rename old DB to {}", backup_path))?;
    println!("Old database renamed to {}", backup_path);

    // Rename new database to the original path
    std::fs::rename(NEW_DB_PATH, OLD_DB_PATH)
        .context("Failed to rename new DB to original path")?;
    println!("New database moved to {}", OLD_DB_PATH);

    Ok(())
}
