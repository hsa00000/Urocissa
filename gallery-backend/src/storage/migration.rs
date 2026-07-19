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
const V4_DB_NAME: &str = "index_v4.redb";
const RECORD_BATCH_SIZE: usize = 16_384;
const LEGACY_TABLE_NAME: &str = "database";

pub fn prepare_storage() -> Result<()> {
    prepare_storage_at(&get_data_path().join("db"))
}

fn prepare_storage_at(db_dir: &Path) -> Result<()> {
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
        return Ok(());
    }

    let v4 = db_dir.join(V4_DB_NAME);
    if v4.exists() {
        bail!(
            "Old database format detected at {}. Please downgrade Urocissa to version 1.2.2, let it migrate the database to V5, then upgrade again.",
            v4.display()
        );
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
            writer.insert_v6_at(&key, &value)?;
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
        collections::{BTreeMap, HashSet},
        env,
        process::Command,
    };

    use arrayvec::ArrayString;
    use redb::{Database, Durability, ReadOnlyDatabase};
    use tempfile::tempdir;

    use super::*;
    use crate::storage::legacy_v5::{
        LegacyFileModify, LegacyImageCombined, LegacyImageMetadata, LegacyObjectSchema,
        LegacyObjectType,
    };

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

    fn create_v5(path: &Path) -> Result<ArrayString<64>> {
        let (id, value) = legacy_image_fixture();
        let db = Database::create(path)?;
        let txn = db.begin_write()?;
        {
            let mut table = txn.open_table(TableDefinition::<&str, LegacyAbstractData>::new(
                LEGACY_TABLE_NAME,
            ))?;
            table.insert(id.as_str(), value)?;
        }
        txn.commit()?;
        drop(db);
        Ok(id)
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
        assert_eq!(store.read(|reader| reader.len())?, 0);
        Ok(())
    }

    #[test]
    fn unversioned_database_is_ignored() -> Result<()> {
        let directory = tempdir()?;
        fs::write(directory.path().join("index.redb"), b"ignored")?;

        prepare_storage_at(directory.path())?;

        assert_eq!(fs::read(directory.path().join("index.redb"))?, b"ignored");
        assert!(directory.path().join(V6_DB_NAME).exists());
        Ok(())
    }

    #[test]
    fn v6_table_type_is_checked_when_opened() -> Result<()> {
        let directory = tempdir()?;
        let path = directory.path().join(V6_DB_NAME);
        let db = Database::create(&path)?;
        let txn = db.begin_write()?;
        {
            let _wrong = txn.open_table(TableDefinition::<&str, &str>::new("records"))?;
        }
        txn.commit()?;
        drop(db);

        let store = DataStore::open(&path)?;
        assert!(store.read(|reader| reader.len()).is_err());
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
