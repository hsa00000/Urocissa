use crate::public::redb::SCHEMA_TABLE;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::{error, info, warn};
use redb::ReadableTable;
use redb::ReadableTableMetadata;
use redb::WriteTransaction;
use std::fs;
use std::io;
use std::io::Write;

pub fn check_database_schema_version() {
    // Check if the database directory or file exists
    if !fs::metadata("./db/index.redb").is_ok() {
        warn!("Database not found. Creating a new one.");
        let txn = crate::public::tree::TREE.in_disk.begin_write().unwrap();
        migration_version(&txn);
        txn.commit()
            .expect("Migration failed: Unable to commit transaction; database rolled back.");
        return;
    }

    let version = read_version();

    info!("Database schema version: {}", version);

    if version == 0 {
        warn!("Performing database migration.");
        warn!(
            "Please BACKUP your database file: Urocissa/gallery-backend/db/index.redb before proceeding."
        );
        warn!("Migration has risks and may modify the database schema.");
        warn!("If migration fails or destroys the database, you can recover from your backup.");
        print!("Have you backed up your database? (Y/n): ");
        io::stdout().flush().expect("Failed to flush stdout"); // Replaced unwrap with expect

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");

        let input = input.trim().to_lowercase();
        if input != "y" {
            panic!("Migration aborted. Please backup your database first.");
        }

        migration();
        info!("Database migration completed.");
    }
}

pub fn read_version() -> u8 {
    let database = redb::Database::create("./db/index.redb")
        .expect("Failed to create or open the database at './db/index.redb'");
    let version: u8;
    let txn = database
        .begin_write()
        .expect("Failed to begin a write transaction on the database");
    {
        let version_table = txn
            .open_table(SCHEMA_TABLE)
            .expect("Failed to open the schema table in the database");

        // Read "version" or default to 0
        version = version_table
            .get("version")
            .expect("Failed to retrieve 'version' from the schema table")
            .map(|guard| guard.value())
            .unwrap_or(0);
    }
    txn.commit()
        .expect("Failed to commit the transaction while reading the version");
    version
}

pub fn migration() {
    let database = redb::Database::create("./db/index.redb")
        .expect("Migration failed: Unable to create or open the database");

    let txn = database
        .begin_write()
        .expect("Migration failed: Unable to begin write transaction");

    migration_database(&txn);
    migration_album(&txn);
    migration_version(&txn);

    txn.commit()
        .expect("Migration failed: Unable to commit transaction; database rolled back.");
}

pub fn migration_database(txn: &WriteTransaction) {
    let mut new_table = txn
        .open_table(crate::public::redb::DATA_TABLE)
        .expect("Migration failed: Unable to open the new data table");
    let old_table = txn
        .open_table(urocissa::DATA_TABLE)
        .expect("Migration failed: Unable to open the old data table");

    let progress_bar = ProgressBar::new(
        old_table
            .len()
            .expect("Migration failed: Cannot get the length of the old data table") as u64,
    );
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
            .expect("Failed to set progress bar template") // Replaced unwrap with expect
            .progress_chars("##-"),
    );

    progress_bar.set_message("Migrating database...");

    old_table
        .iter()
        .expect("Migration failed: Unable to iterate over the old data table")
        .for_each(|result| {
            let (_, value_guard) =
                result.expect("Migration failed: Unable to retrieve value from the old data table");
            let old_data = value_guard.value();
            let converted = crate::public::database_struct::database::definition::Database {
                hash: old_data.hash,
                size: old_data.size,
                width: old_data.width,
                height: old_data.height,
                thumbhash: old_data.thumbhash,
                phash: old_data.phash,
                ext: old_data.ext,
                exif_vec: old_data.exif_vec,
                tag: old_data.tag,
                album: old_data.album,
                alias: old_data
                    .alias
                    .into_iter()
                    .map(
                        |old_alias| crate::public::database_struct::file_modify::FileModify {
                            file: old_alias.file,
                            modified: old_alias.modified,
                            scan_time: old_alias.scan_time,
                        },
                    )
                    .collect(),
                ext_type: old_data.ext_type,
                pending: old_data.pending,
            };
            new_table
                .insert(&*old_data.hash, converted)
                .expect("Migration failed: Unable to insert data into the new data table");
            progress_bar.inc(1);
        });

    progress_bar.finish_with_message("Migrating database completed");

    txn.delete_table(old_table)
        .expect("Migration failed: Unable to delete the old data table");
}

pub fn migration_album(txn: &WriteTransaction) {
    let mut new_table = txn
        .open_table(crate::public::redb::ALBUM_TABLE)
        .expect("Migration failed: Unable to open the new album table");
    let old_table = txn
        .open_table(urocissa::ALBUM_TABLE)
        .expect("Migration failed: Unable to open the old album table");

    let database_table = txn
        .open_table(crate::public::redb::DATA_TABLE)
        .expect("Migration failed: Unable to open the data table");

    let progress_bar = ProgressBar::new(
        old_table
            .len()
            .expect("Migration failed: Cannot get the length of the old album table")
            as u64,
    );
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
            .expect("Failed to set progress bar template") // Replaced unwrap with expect
            .progress_chars("##-"),
    );

    progress_bar.set_message("Migrating album...");

    old_table
        .iter()
        .expect("Migration failed: Unable to iterate over the old album table")
        .for_each(|result| {
            let (_, value_guard) = result
                .expect("Migration failed: Unable to retrieve value from the old album table");
            let old_album = value_guard.value();

            let thumbhash_opt =
                old_album
                    .cover
                    .and_then(|cover_hash| match database_table.get(&*cover_hash) {
                        Ok(Some(guard)) => Some(guard.value().thumbhash),
                        Ok(None) => panic!(
                            "Migration failed: Album {} cannot get cover {}",
                            old_album.id, cover_hash
                        ),
                        Err(err) => {
                            error!("{}", err);
                            panic!(
                                "Migration failed: Album {} cannot get thumbhash with cover {}",
                                old_album.id, cover_hash
                            );
                        }
                    });

            let converted = crate::public::album::Album {
                id: old_album.id,
                title: old_album.title,
                created_time: old_album.created_time,
                start_time: old_album.start_time,
                end_time: old_album.end_time,
                last_modified_time: old_album.last_modified_time,
                cover: old_album.cover,
                thumbhash: thumbhash_opt,
                user_defined_metadata: old_album.user_defined_metadata,
                share_list: old_album
                    .share_list
                    .into_iter()
                    .map(|share| crate::public::album::Share {
                        url: share.url,
                        description: share.description,
                        password: share.password,
                        show_metadata: share.show_metadata,
                        show_download: share.show_download,
                        show_upload: share.show_upload,
                        exp: share.exp,
                    })
                    .collect(),
                tag: old_album.tag,
                width: old_album.width,
                height: old_album.height,
                item_count: old_album.item_count,
                item_size: old_album.item_size,
                pending: old_album.pending,
            };
            new_table
                .insert(&*old_album.id, converted)
                .expect("Migration failed: Unable to insert data into the new album table");
            progress_bar.inc(1);
        });

    progress_bar.finish_with_message("Migrating album completed");

    txn.delete_table(old_table)
        .expect("Migration failed: Unable to delete the old album table");
}

pub fn migration_version(txn: &WriteTransaction) {
    let mut version_table = txn
        .open_table(SCHEMA_TABLE)
        .expect("Migration failed: Unable to open the schema table");
    version_table
        .insert("version", 1)
        .expect("Migration failed: Unable to insert version number into the schema table");
    info!("Database schema updated to version 1");
}
