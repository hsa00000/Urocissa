use std::{
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use redb::{
    DatabaseError, Durability, ReadableDatabase, ReadableTable, ReadableTableMetadata,
    TableDefinition,
};

use super::{
    cache::{CacheClass, database_builder},
    legacy_v5::LegacyAbstractData,
    store::DataStore,
    v6::V6AbstractData,
};
use crate::public::constant::storage::get_data_path;

const V5_DB_NAME: &str = "index_v5.redb";
const V6_DB_NAME: &str = "index_v6.redb";
const MIGRATING_DB_NAME: &str = "index_v6.redb.migrating";
const RECORD_BATCH_SIZE: usize = 16_384;
const LEGACY_TABLE_NAME: &str = "database";

pub fn prepare_storage() -> Result<()> {
    prepare_storage_at(&get_data_path().join("db"))
}

pub(crate) fn prepare_storage_at(db_dir: &Path) -> Result<()> {
    fs::create_dir_all(db_dir)
        .with_context(|| format!("failed to create database directory {}", db_dir.display()))?;

    let current = db_dir.join(V6_DB_NAME);
    if current.exists() {
        return Ok(());
    }

    let migrating = db_dir.join(MIGRATING_DB_NAME);
    if migrating.exists() {
        info!("Removing incomplete V6 migration before restarting");
        remove_file(&migrating, "incomplete V6 migration")?;
    }

    let v5 = db_dir.join(V5_DB_NAME);
    if v5.exists() {
        info!("Migrating V5 bitcode database to frozen V6 bitcode storage");
        migrate_v5(&v5, &migrating, &current)?;
        info!(
            "V5 migration completed; V6 database is ready at {}",
            current.display()
        );
        return Ok(());
    }

    // Experimental unversioned files are intentionally ignored. Only the
    // explicitly versioned V5 and V6 paths participate in storage selection.
    info!("Creating a new empty V6 database at {}", current.display());
    drop(DataStore::initialize_empty(&current)?);
    Ok(())
}

fn migrate_v5(source_path: &Path, migrating_path: &Path, current_path: &Path) -> Result<()> {
    let result = match database_builder(CacheClass::Migration).open_read_only(source_path) {
        Ok(old_db) => migrate_v5_from_database(&old_db, migrating_path, current_path),
        Err(DatabaseError::RepairAborted) => {
            warn!("V5 database requires repair; repairing index_v5.redb in place");
            let repaired_db = database_builder(CacheClass::Migration)
                .open(source_path)
                .with_context(|| {
                    format!(
                        "failed to repair V5 database in place {}",
                        source_path.display()
                    )
                })?;
            migrate_v5_from_database(&repaired_db, migrating_path, current_path)
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to open V5 database {}", source_path.display())),
    };

    if result.is_err()
        && migrating_path.exists()
        && let Err(error) = fs::remove_file(migrating_path)
    {
        warn!(
            "Failed to remove incomplete V6 migration {}: {error}",
            migrating_path.display()
        );
    }
    result
}

fn migrate_v5_from_database(
    old_db: &impl ReadableDatabase,
    migrating_path: &Path,
    current_path: &Path,
) -> Result<()> {
    let read_txn = old_db.begin_read()?;
    let old_table = read_txn
        .open_table(TableDefinition::<&str, LegacyAbstractData>::new(
            LEGACY_TABLE_NAME,
        ))
        .with_context(|| format!("failed to open V5 table {LEGACY_TABLE_NAME}"))?;
    let expected_count = old_table.len()?;
    let store = DataStore::initialize_empty(migrating_path)?;
    let started = Instant::now();
    let mut processed = 0_u64;
    let mut batch = Vec::with_capacity(RECORD_BATCH_SIZE);

    for entry in old_table.iter()? {
        let (key, value) = entry?;
        let key = key.value().to_owned();
        let legacy = catch_unwind(AssertUnwindSafe(|| value.value()))
            .map_err(|_| anyhow!("failed to decode V5 record {key}: bitcode payload is invalid"))?;
        batch.push((key, legacy));

        if batch.len() == RECORD_BATCH_SIZE {
            processed += migrate_batch(&store, std::mem::take(&mut batch))?;
            log_progress(processed, expected_count, started);
        }
    }

    if !batch.is_empty() {
        processed += migrate_batch(&store, batch)?;
        log_progress(processed, expected_count, started);
    }

    if processed != expected_count {
        bail!("V6 migration count mismatch: processed {processed}, expected {expected_count}");
    }

    let destination_count = store.record_count()?;
    if destination_count != expected_count {
        bail!("V6 destination count mismatch: {destination_count}, expected {expected_count}");
    }

    // This Immediate commit persists all preceding Durability::None batches.
    store.sync()?;
    drop(store);
    drop(old_table);
    drop(read_txn);

    if current_path.exists() {
        bail!(
            "refusing to overwrite existing V6 database {}",
            current_path.display()
        );
    }
    fs::rename(migrating_path, current_path).with_context(|| {
        format!(
            "failed to finalize V6 migration from {} to {}",
            migrating_path.display(),
            current_path.display()
        )
    })?;
    info!(
        "V5 to V6 migration completed: {processed} records in {:.2}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn migrate_batch(store: &DataStore, batch: Vec<(String, LegacyAbstractData)>) -> Result<u64> {
    let count = batch.len() as u64;
    store.write_with_durability(Durability::None, |writer| {
        for (key, legacy) in batch {
            let value = V6AbstractData::from_v5(legacy)
                .with_context(|| format!("failed to convert V5 record {key}"))?;
            writer.insert_v6_at(&key, value)?;
        }
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(count)
}

fn log_progress(processed: u64, total: u64, started: Instant) {
    if processed != total && !processed.is_multiple_of((RECORD_BATCH_SIZE * 8) as u64) {
        return;
    }
    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    #[allow(clippy::cast_precision_loss)]
    let rate = processed as f64 / elapsed;
    info!("Migrating V5 to V6: {processed}/{total}, {rate:.0} records/s");
}

fn remove_file(path: &Path, description: &str) -> Result<()> {
    fs::remove_file(path)
        .with_context(|| format!("failed to remove {description} {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, HashMap, HashSet},
        env,
        process::Command,
    };

    use arrayvec::ArrayString;
    use redb::{Database, Durability, ReadOnlyDatabase, TypeName, Value};
    use tempfile::tempdir;

    use super::*;
    use crate::storage::legacy_v5::{
        LegacyAlbumCombined, LegacyAlbumMetadata, LegacyFileModify, LegacyImageCombined,
        LegacyImageMetadata, LegacyObjectSchema, LegacyObjectType, LegacyShare,
        LegacyVideoCombined, LegacyVideoMetadata,
    };

    #[derive(Debug)]
    struct RawV5Bytes;

    impl Value for RawV5Bytes {
        type SelfType<'a> = &'a [u8];
        type AsBytes<'a> = &'a [u8];

        fn fixed_width() -> Option<usize> {
            None
        }

        fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
        where
            Self: 'a,
        {
            data
        }

        fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
            value
        }

        fn type_name() -> TypeName {
            TypeName::new("AbstractData")
        }
    }

    fn legacy_image_fixture() -> (ArrayString<64>, LegacyAbstractData) {
        let id = ArrayString::<64>::from("image-1").unwrap();
        let value = LegacyAbstractData::Image(LegacyImageCombined {
            object: LegacyObjectSchema {
                id,
                obj_type: LegacyObjectType::Image,
                pending: false,
                thumbhash: Some(vec![1, 2, 3]),
                description: Some("fixture".to_owned()),
                tags: HashSet::from(["tag".to_owned()]),
                is_favorite: true,
                is_archived: false,
                is_trashed: false,
                update_at: 123,
            },
            metadata: LegacyImageMetadata {
                id,
                size: 42,
                width: 100,
                height: 80,
                ext: "jpg".to_owned(),
                phash: Some(vec![4, 5]),
                albums: HashSet::new(),
                exif_vec: BTreeMap::from([("Make".to_owned(), "Camera".to_owned())]),
                alias: vec![LegacyFileModify {
                    file: "fixture.jpg".to_owned(),
                    modified: 1,
                    scan_time: 2,
                }],
            },
        });
        (id, value)
    }

    fn legacy_video_fixture() -> (ArrayString<64>, LegacyAbstractData) {
        let id = ArrayString::<64>::from("video-1").unwrap();
        let album_id = ArrayString::<64>::from("album-1").unwrap();
        let value = LegacyAbstractData::Video(LegacyVideoCombined {
            object: LegacyObjectSchema {
                id,
                obj_type: LegacyObjectType::Video,
                pending: true,
                thumbhash: Some(vec![6, 7]),
                description: Some("video fixture".to_owned()),
                tags: HashSet::from(["motion".to_owned()]),
                is_favorite: false,
                is_archived: true,
                is_trashed: false,
                update_at: 456,
            },
            metadata: LegacyVideoMetadata {
                id,
                size: 84,
                width: 1_920,
                height: 1_080,
                ext: "mp4".to_owned(),
                duration: 12.5,
                albums: HashSet::from([album_id]),
                exif_vec: BTreeMap::from([("rotation".to_owned(), "90".to_owned())]),
                alias: vec![LegacyFileModify {
                    file: "fixture.mp4".to_owned(),
                    modified: 3,
                    scan_time: 4,
                }],
            },
        });
        (id, value)
    }

    fn legacy_album_fixture() -> (ArrayString<64>, LegacyAbstractData) {
        let id = ArrayString::<64>::from("album-1").unwrap();
        let image_id = ArrayString::<64>::from("image-1").unwrap();
        let share_id = ArrayString::<64>::from("share-1").unwrap();
        let share_url = ArrayString::<64>::from("public-share").unwrap();
        let value = LegacyAbstractData::Album(LegacyAlbumCombined {
            object: LegacyObjectSchema {
                id,
                obj_type: LegacyObjectType::Album,
                pending: false,
                thumbhash: None,
                description: Some("album fixture".to_owned()),
                tags: HashSet::from(["collection".to_owned()]),
                is_favorite: true,
                is_archived: false,
                is_trashed: false,
                update_at: 789,
            },
            metadata: LegacyAlbumMetadata {
                id,
                title: Some("Fixture album".to_owned()),
                created_time: 10,
                start_time: Some(11),
                end_time: Some(12),
                last_modified_time: 13,
                cover: Some(image_id),
                item_count: 2,
                item_size: 126,
                share_list: HashMap::from([(
                    share_id,
                    LegacyShare {
                        url: share_url,
                        description: "shared fixture".to_owned(),
                        password: Some("secret".to_owned()),
                        show_metadata: true,
                        show_download: false,
                        show_upload: true,
                        exp: 999,
                    },
                )]),
            },
        });
        (id, value)
    }

    fn create_v5(path: &Path) -> Result<ArrayString<64>> {
        let (image_id, image) = legacy_image_fixture();
        let (video_id, video) = legacy_video_fixture();
        let (album_id, album) = legacy_album_fixture();
        let db = Database::create(path)?;
        let txn = db.begin_write()?;
        {
            let mut table = txn.open_table(TableDefinition::<&str, LegacyAbstractData>::new(
                LEGACY_TABLE_NAME,
            ))?;
            table.insert(image_id.as_str(), image)?;
            table.insert(video_id.as_str(), video)?;
            table.insert(album_id.as_str(), album)?;
        }
        txn.commit()?;
        drop(db);
        Ok(image_id)
    }

    #[test]
    fn creates_an_empty_v6_database() -> Result<()> {
        let directory = tempdir()?;
        prepare_storage_at(directory.path())?;
        let current = directory.path().join(V6_DB_NAME);
        let store = DataStore::open(&current)?;
        assert_eq!(store.record_count()?, 0);
        Ok(())
    }

    #[test]
    fn v6_table_len_is_the_authoritative_record_count() -> Result<()> {
        let directory = tempdir()?;
        let store = DataStore::initialize_empty(&directory.path().join(V6_DB_NAME))?;
        let (id, legacy) = legacy_image_fixture();
        let value = V6AbstractData::from_v5(legacy)?.into_domain()?;

        assert_eq!(store.record_count()?, 0);
        store.write(|writer| writer.insert(&value))?;
        assert_eq!(store.record_count()?, 1);
        store.write(|writer| writer.remove(id.as_str()))?;
        assert_eq!(store.record_count()?, 0);
        Ok(())
    }

    #[test]
    fn migrates_v5_directly_to_v6_and_keeps_source() -> Result<()> {
        let directory = tempdir()?;
        let source = directory.path().join(V5_DB_NAME);
        let id = create_v5(&source)?;
        let source_before = fs::read(&source)?;

        prepare_storage_at(directory.path())?;

        assert_eq!(fs::read(&source)?, source_before);
        let store = DataStore::open(&directory.path().join(V6_DB_NAME))?;
        let value = store
            .read(|reader| reader.get(id.as_str()))?
            .expect("migrated image")
            .into_value();
        assert_eq!(value.hash(), id);
        assert_eq!(value.cache_version(), 0);

        let video = store
            .read(|reader| reader.get("video-1"))?
            .expect("migrated video")
            .into_value();
        let crate::public::structure::abstract_data::AbstractData::Video(video) = video else {
            panic!("video-1 changed variant during migration");
        };
        assert_eq!(video.object.cache_version, 0);
        assert_eq!(video.object.description.as_deref(), Some("video fixture"));
        assert!((video.metadata.duration - 12.5).abs() < f64::EPSILON);
        assert!(video.metadata.albums.contains("album-1"));

        let album = store
            .read(|reader| reader.get("album-1"))?
            .expect("migrated album")
            .into_value();
        let crate::public::structure::abstract_data::AbstractData::Album(album) = album else {
            panic!("album-1 changed variant during migration");
        };
        assert_eq!(album.object.cache_version, 0);
        assert_eq!(album.metadata.title.as_deref(), Some("Fixture album"));
        assert_eq!(album.metadata.item_count, 2);
        let share_id = ArrayString::<64>::from("share-1").unwrap();
        let share = album.metadata.share_list.get(&share_id).unwrap();
        assert_eq!(share.url.as_str(), "public-share");
        assert_eq!(share.password.as_deref(), Some("secret"));
        Ok(())
    }

    #[test]
    fn stale_migration_is_discarded_and_restarted() -> Result<()> {
        let directory = tempdir()?;
        create_v5(&directory.path().join(V5_DB_NAME))?;
        fs::write(directory.path().join(MIGRATING_DB_NAME), b"stale")?;

        prepare_storage_at(directory.path())?;

        assert!(!directory.path().join(MIGRATING_DB_NAME).exists());
        assert!(directory.path().join(V6_DB_NAME).exists());
        Ok(())
    }

    #[test]
    fn existing_v6_always_wins_over_v5() -> Result<()> {
        let directory = tempdir()?;
        create_v5(&directory.path().join(V5_DB_NAME))?;
        drop(DataStore::initialize_empty(
            &directory.path().join(V6_DB_NAME),
        )?);

        prepare_storage_at(directory.path())?;

        let store = DataStore::open(&directory.path().join(V6_DB_NAME))?;
        assert_eq!(store.record_count()?, 0);
        Ok(())
    }

    #[test]
    fn unversioned_database_is_ignored() -> Result<()> {
        let directory = tempdir()?;
        fs::write(directory.path().join("index.redb"), b"ignored")?;
        fs::write(directory.path().join("index_v4.redb"), b"also ignored")?;

        prepare_storage_at(directory.path())?;

        assert_eq!(fs::read(directory.path().join("index.redb"))?, b"ignored");
        assert_eq!(
            fs::read(directory.path().join("index_v4.redb"))?,
            b"also ignored"
        );
        assert!(directory.path().join(V6_DB_NAME).exists());
        Ok(())
    }

    #[test]
    fn migration_decode_error_removes_temporary_v6_and_reports_key() -> Result<()> {
        let directory = tempdir()?;
        let source = directory.path().join(V5_DB_NAME);
        let database = Database::create(&source)?;
        let transaction = database.begin_write()?;
        {
            let mut table = transaction
                .open_table(TableDefinition::<&str, RawV5Bytes>::new(LEGACY_TABLE_NAME))?;
            table.insert("broken-v5-record", b"not-valid-bitcode".as_slice())?;
        }
        transaction.commit()?;
        drop(database);

        let error = prepare_storage_at(directory.path()).unwrap_err();
        assert!(format!("{error:#}").contains("broken-v5-record"));
        assert!(source.exists());
        assert!(!directory.path().join(MIGRATING_DB_NAME).exists());
        assert!(!directory.path().join(V6_DB_NAME).exists());
        Ok(())
    }

    #[test]
    fn v6_table_type_is_checked_when_opened() -> Result<()> {
        let directory = tempdir()?;
        create_v5(&directory.path().join(V5_DB_NAME))?;
        let path = directory.path().join(V6_DB_NAME);
        let db = Database::create(&path)?;
        let txn = db.begin_write()?;
        {
            let _wrong = txn.open_table(TableDefinition::<&str, &str>::new("records"))?;
        }
        txn.commit()?;
        drop(db);

        prepare_storage_at(directory.path())?;
        let store = DataStore::open(&path)?;
        assert!(store.record_count().is_err());
        assert!(directory.path().join(V5_DB_NAME).exists());
        Ok(())
    }
    const UNCLEAN_V5_FIXTURE_PATH_ENV: &str = "UROCISSA_TEST_UNCLEAN_V5_PATH";

    #[test]
    fn repairs_unclean_v5_in_place_before_migrating() -> Result<()> {
        if let Some(path) = env::var_os(UNCLEAN_V5_FIXTURE_PATH_ENV) {
            let (id, value) = legacy_image_fixture();
            let database = Database::create(Path::new(&path))?;
            let transaction = database.begin_write()?;
            {
                let mut table = transaction
                    .open_table(TableDefinition::<&str, LegacyAbstractData>::new(
                        LEGACY_TABLE_NAME,
                    ))?;
                table.insert(id.as_str(), value)?;
            }
            transaction.commit()?;

            let mut dirty_transaction = database.begin_write()?;
            dirty_transaction.set_durability(Durability::None)?;
            {
                let mut table = dirty_transaction
                    .open_table(TableDefinition::<&str, &str>::new("dirty-marker"))?;
                table.insert("marker", "unclean")?;
            }
            dirty_transaction.commit()?;
            std::process::exit(0);
        }

        let directory = tempdir()?;
        let source = directory.path().join(V5_DB_NAME);
        let status = Command::new(env::current_exe()?)
            .arg("--exact")
            .arg("storage::migration::tests::repairs_unclean_v5_in_place_before_migrating")
            .arg("--nocapture")
            .env(UNCLEAN_V5_FIXTURE_PATH_ENV, &source)
            .status()?;
        assert!(status.success());

        assert!(matches!(
            ReadOnlyDatabase::open(&source),
            Err(DatabaseError::RepairAborted)
        ));

        prepare_storage_at(directory.path())?;

        assert!(source.exists());
        assert!(!directory.path().join(MIGRATING_DB_NAME).exists());
        assert!(ReadOnlyDatabase::open(&source).is_ok());
        let (id, _) = legacy_image_fixture();
        let store = DataStore::open(&directory.path().join(V6_DB_NAME))?;
        let value = store
            .read(|reader| reader.get(id.as_str()))?
            .expect("migrated image")
            .into_value();
        assert_eq!(value.hash(), id);
        Ok(())
    }
}
