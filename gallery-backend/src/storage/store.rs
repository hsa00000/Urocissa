use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use bitcode::Buffer;
use redb::{
    Database, Durability, ReadOnlyTable, ReadableDatabase, ReadableTable, ReadableTableMetadata,
    Table, TableDefinition,
};

use super::{
    cache::{CacheClass, database_builder},
    v6::{RawV6Bytes, V6AbstractData},
};
use crate::public::structure::abstract_data::AbstractData;

pub(crate) const RECORDS_TABLE: TableDefinition<&str, RawV6Bytes> = TableDefinition::new("records");

pub struct DataStore {
    db: Database,
    #[cfg(feature = "performance-test")]
    cache_limit_bytes: usize,
}

pub struct RecordReader {
    table: ReadOnlyTable<&'static str, RawV6Bytes>,
}

pub struct RecordIter<'a> {
    inner: redb::Range<'a, &'static str, RawV6Bytes>,
    codec: Buffer,
}

pub struct RecordValuesIter<'a> {
    inner: redb::Range<'a, &'static str, RawV6Bytes>,
    codec: Buffer,
}

#[derive(Debug)]
pub struct RecordKey(String);

#[derive(Debug)]
pub struct RecordValue(AbstractData);

impl RecordKey {
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl RecordValue {
    pub fn into_value(self) -> AbstractData {
        self.0
    }
}

pub struct RecordWriter<'txn> {
    table: Table<'txn, &'static str, RawV6Bytes>,
    codec: Buffer,
    io_timing: RecordIoTiming,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RecordWriteTiming {
    pub decode: Duration,
    pub encode_insert: Duration,
    pub commit: Duration,
}

type RecordIoTiming = RecordWriteTiming;

impl DataStore {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_cache(path, CacheClass::Main)
    }

    pub(crate) fn open_with_cache(path: &Path, cache_class: CacheClass) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create database directory {}", parent.display())
            })?;
        }
        Ok(Self {
            db: database_builder(cache_class)
                .create(path)
                .with_context(|| format!("failed to open V6 database {}", path.display()))?,
            #[cfg(feature = "performance-test")]
            cache_limit_bytes: cache_class.limit_bytes(),
        })
    }

    #[cfg(feature = "performance-test")]
    pub const fn cache_limit_bytes(&self) -> usize {
        self.cache_limit_bytes
    }

    #[cfg(feature = "performance-test")]
    pub fn cache_stats(&self) -> redb::CacheStats {
        self.db.cache_stats()
    }

    pub fn initialize_empty(path: &Path) -> Result<Self> {
        let store = Self::open(path)?;
        let mut txn = store.db.begin_write()?;
        txn.set_durability(Durability::Immediate)?;
        {
            let _records = txn.open_table(RECORDS_TABLE)?;
        }
        txn.commit()?;
        Ok(store)
    }

    pub fn read<R, E>(
        &self,
        operation: impl FnOnce(&RecordReader) -> std::result::Result<R, E>,
    ) -> std::result::Result<R, E>
    where
        E: From<anyhow::Error>,
    {
        let txn = self
            .db
            .begin_read()
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        let table = txn
            .open_table(RECORDS_TABLE)
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        operation(&RecordReader { table })
    }

    pub fn reader(&self) -> Result<RecordReader> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(RECORDS_TABLE)?;
        Ok(RecordReader { table })
    }

    /// Return redb's authoritative O(1) record count without decoding records.
    pub fn record_count(&self) -> Result<u64> {
        self.read(RecordReader::len)
    }

    pub fn write<R, E>(
        &self,
        operation: impl FnOnce(&mut RecordWriter<'_>) -> std::result::Result<R, E>,
    ) -> std::result::Result<R, E>
    where
        E: From<anyhow::Error>,
    {
        self.write_with_durability(Durability::Immediate, operation)
    }

    pub fn write_with_durability<R, E>(
        &self,
        durability: Durability,
        operation: impl FnOnce(&mut RecordWriter<'_>) -> std::result::Result<R, E>,
    ) -> std::result::Result<R, E>
    where
        E: From<anyhow::Error>,
    {
        let mut txn = self
            .db
            .begin_write()
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        txn.set_durability(durability)
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        let table = txn
            .open_table(RECORDS_TABLE)
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        let mut writer = RecordWriter {
            table,
            codec: Buffer::new(),
            io_timing: RecordIoTiming::default(),
        };
        let result = operation(&mut writer)?;
        drop(writer);
        txn.commit().map_err(anyhow::Error::from).map_err(E::from)?;
        Ok(result)
    }

    pub(crate) fn write_profiled<R, E>(
        &self,
        operation: impl FnOnce(&mut RecordWriter<'_>) -> std::result::Result<R, E>,
    ) -> std::result::Result<(R, RecordWriteTiming), E>
    where
        E: From<anyhow::Error>,
    {
        let mut txn = self
            .db
            .begin_write()
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        txn.set_durability(Durability::Immediate)
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        let table = txn
            .open_table(RECORDS_TABLE)
            .map_err(anyhow::Error::from)
            .map_err(E::from)?;
        let mut writer = RecordWriter {
            table,
            codec: Buffer::new(),
            io_timing: RecordIoTiming::default(),
        };
        let result = operation(&mut writer)?;
        let mut timing = writer.io_timing;
        drop(writer);
        let commit_started = Instant::now();
        txn.commit().map_err(anyhow::Error::from).map_err(E::from)?;
        timing.commit = commit_started.elapsed();
        Ok((result, timing))
    }

    /// Persist all preceding `Durability::None` migration commits.
    pub fn sync(&self) -> Result<()> {
        let mut txn = self.db.begin_write()?;
        txn.set_durability(Durability::Immediate)?;
        {
            let _records = txn.open_table(RECORDS_TABLE)?;
        }
        txn.commit()?;
        Ok(())
    }
}

impl RecordReader {
    pub fn len(&self) -> Result<u64> {
        Ok(self.table.len()?)
    }

    pub fn get(&self, key: &str) -> Result<Option<RecordValue>> {
        let Some(value) = self.table.get(key)? else {
            return Ok(None);
        };
        let value = bitcode::decode::<V6AbstractData>(value.value())
            .map_err(|error| anyhow!("failed to decode V6 record {key}: {error}"))?
            .into_domain()
            .with_context(|| format!("failed to convert V6 record {key}"))?;
        Ok(Some(RecordValue(value)))
    }

    #[allow(clippy::iter_not_returning_iterator)]
    pub fn iter(&self) -> Result<RecordIter<'_>> {
        Ok(RecordIter {
            inner: self.table.iter()?,
            codec: Buffer::new(),
        })
    }

    /// Iterate decoded values without allocating an owned key for every row.
    ///
    /// The borrowed Redb key remains available while decoding so corrupt
    /// payload and conversion errors still identify the exact record.
    #[allow(clippy::iter_not_returning_iterator)]
    pub fn values(&self) -> Result<RecordValuesIter<'_>> {
        Ok(RecordValuesIter {
            inner: self.table.iter()?,
            codec: Buffer::new(),
        })
    }
}

impl Iterator for RecordIter<'_> {
    type Item = Result<(RecordKey, RecordValue)>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.inner.next()?;
        Some(self.decode_entry(entry))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for RecordIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let entry = self.inner.next_back()?;
        Some(self.decode_entry(entry))
    }
}

impl RecordIter<'_> {
    fn decode_entry(
        &mut self,
        entry: redb::Result<(
            redb::AccessGuard<'_, &'static str>,
            redb::AccessGuard<'_, RawV6Bytes>,
        )>,
    ) -> Result<(RecordKey, RecordValue)> {
        let (key, value) = entry?;
        let key = key.value().to_owned();
        let value = self
            .codec
            .decode::<V6AbstractData>(value.value())
            .map_err(|error| anyhow!("failed to decode V6 record {key}: {error}"))?
            .into_domain()
            .with_context(|| format!("failed to convert V6 record {key}"))?;
        Ok((RecordKey(key), RecordValue(value)))
    }
}

impl Iterator for RecordValuesIter<'_> {
    type Item = Result<RecordValue>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.inner.next()?;
        Some(self.decode_entry(entry))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for RecordValuesIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let entry = self.inner.next_back()?;
        Some(self.decode_entry(entry))
    }
}

impl RecordValuesIter<'_> {
    fn decode_entry(
        &mut self,
        entry: redb::Result<(
            redb::AccessGuard<'_, &'static str>,
            redb::AccessGuard<'_, RawV6Bytes>,
        )>,
    ) -> Result<RecordValue> {
        let (key, value) = entry?;
        let key = key.value();
        let value = self
            .codec
            .decode::<V6AbstractData>(value.value())
            .map_err(|error| anyhow!("failed to decode V6 record {key}: {error}"))?
            .into_domain()
            .with_context(|| format!("failed to convert V6 record {key}"))?;
        Ok(RecordValue(value))
    }
}

impl RecordWriter<'_> {
    pub fn get(&mut self, key: &str) -> Result<Option<RecordValue>> {
        self.get_v6(key)?
            .map(V6AbstractData::into_domain)
            .transpose()
            .with_context(|| format!("failed to convert V6 record {key}"))
            .map(|value| value.map(RecordValue))
    }

    pub fn get_v6(&mut self, key: &str) -> Result<Option<V6AbstractData>> {
        let Some(value) = self.table.get(key)? else {
            return Ok(None);
        };
        let value = self
            .codec
            .decode::<V6AbstractData>(value.value())
            .map_err(|error| anyhow!("failed to decode V6 record {key}: {error}"))?;
        Ok(Some(value))
    }

    pub(crate) fn get_v6_profiled(&mut self, key: &str) -> Result<Option<V6AbstractData>> {
        let started = Instant::now();
        let result = self.get_v6(key);
        self.io_timing.decode = self.io_timing.decode.saturating_add(started.elapsed());
        result
    }

    pub fn insert(&mut self, value: &AbstractData) -> Result<()> {
        let key = value.hash();
        let stored = V6AbstractData::from(value);
        self.insert_v6_at(key.as_str(), stored)
    }

    #[cfg(feature = "performance-test")]
    pub fn insert_owned(&mut self, value: AbstractData) -> Result<()> {
        let key = value.hash();
        self.insert_v6_at(key.as_str(), V6AbstractData::from(value))
    }

    #[cfg(all(test, feature = "performance-test"))]
    pub fn insert_at(&mut self, key: &str, value: &AbstractData) -> Result<()> {
        if value.hash().as_str() != key {
            return Err(anyhow!(
                "record key {key} does not match record id {}",
                value.hash()
            ));
        }
        self.insert_v6_at(key, V6AbstractData::from(value))
    }

    pub fn insert_at_owned(&mut self, key: &str, value: AbstractData) -> Result<()> {
        if value.hash().as_str() != key {
            return Err(anyhow!(
                "record key {key} does not match record id {}",
                value.hash()
            ));
        }
        self.insert_v6_at(key, V6AbstractData::from(value))
    }

    // Ownership is intentional: write callers transfer the converted record
    // instead of retaining it or cloning its dynamic fields.
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_v6_at(&mut self, key: &str, value: V6AbstractData) -> Result<()> {
        if value.id() != key {
            return Err(anyhow!(
                "record key {key} does not match V6 record id {}",
                value.id()
            ));
        }
        let bytes = self.codec.encode(&value);
        self.table.insert(key, bytes)?;
        Ok(())
    }

    pub(crate) fn insert_v6_at_profiled(&mut self, key: &str, value: V6AbstractData) -> Result<()> {
        let started = Instant::now();
        let result = self.insert_v6_at(key, value);
        self.io_timing.encode_insert = self
            .io_timing
            .encode_insert
            .saturating_add(started.elapsed());
        result
    }

    pub fn remove(&mut self, key: &str) -> Result<()> {
        self.table.remove(key)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use arrayvec::ArrayString;
    use redb::{ReadableDatabase, TableDefinition};

    use super::*;
    use crate::storage::v6::{V6ImageCombined, V6ImageMetadata, V6ObjectSchema, V6ObjectType};

    fn fixture(id: &str, description_len: usize) -> V6AbstractData {
        let id = ArrayString::<64>::from(id).unwrap();
        V6AbstractData::Image(V6ImageCombined {
            object: V6ObjectSchema {
                id,
                obj_type: V6ObjectType::Image,
                pending: false,
                thumbhash: Some(vec![1, 2, 3, 4]),
                cache_version: 7,
                description: Some("x".repeat(description_len)),
                tags: HashSet::from(["storage-test".to_owned()]),
                is_favorite: true,
                is_archived: false,
                is_trashed: false,
                update_at: 123,
            },
            metadata: V6ImageMetadata {
                id,
                size: 456,
                width: 10,
                height: 20,
                ext: "webp".to_owned(),
                phash: Some(vec![5, 6, 7]),
                albums: HashSet::new(),
                exif_vec: BTreeMap::new(),
                alias: Vec::new(),
            },
        })
    }

    #[test]
    fn raw_reader_opens_table_written_with_typed_v6_value() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("typed.redb");
        let expected = fixture("typed-record", 8);
        {
            let database = Database::create(&path)?;
            let transaction = database.begin_write()?;
            {
                let definition = TableDefinition::<&str, V6AbstractData>::new("records");
                let mut table = transaction.open_table(definition)?;
                table.insert(expected.id(), expected.clone())?;
            }
            transaction.commit()?;
        }

        let store = DataStore::open(&path)?;
        let actual = store
            .reader()?
            .get(expected.id())?
            .expect("typed record")
            .into_value();
        assert_eq!(V6AbstractData::from(actual), expected);
        Ok(())
    }

    #[test]
    fn typed_reader_opens_table_written_through_raw_store() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("raw.redb");
        let expected = fixture("raw-record", 32);
        {
            let store = DataStore::initialize_empty(&path)?;
            store.write(|writer| writer.insert_v6_at(expected.id(), expected.clone()))?;
        }

        let database = Database::open(&path)?;
        let transaction = database.begin_read()?;
        let definition = TableDefinition::<&str, V6AbstractData>::new("records");
        let table = transaction.open_table(definition)?;
        assert_eq!(table.get(expected.id())?.unwrap().value(), expected);
        Ok(())
    }

    #[test]
    fn writer_reuses_codec_across_different_record_sizes() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("sizes.redb");
        let small = fixture("small-record", 1);
        let large = fixture("large-record", 16 * 1024);
        let store = DataStore::initialize_empty(&path)?;
        store.write(|writer| {
            writer.insert_v6_at(small.id(), small.clone())?;
            writer.insert_v6_at(large.id(), large.clone())?;
            writer.insert_v6_at(small.id(), small.clone())?;
            Ok::<(), anyhow::Error>(())
        })?;

        let reader = store.reader()?;
        assert_eq!(
            V6AbstractData::from(reader.get(small.id())?.unwrap().into_value()),
            small
        );
        assert_eq!(
            V6AbstractData::from(reader.get(large.id())?.unwrap().into_value()),
            large
        );
        Ok(())
    }

    #[test]
    fn values_iterator_matches_keyed_iterator_without_losing_order() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("values.redb");
        let first = fixture("a-values-record", 4);
        let second = fixture("b-values-record", 64);
        let store = DataStore::initialize_empty(&path)?;
        store.write(|writer| {
            writer.insert_v6_at(first.id(), first.clone())?;
            writer.insert_v6_at(second.id(), second.clone())?;
            Ok::<(), anyhow::Error>(())
        })?;

        let reader = store.reader()?;
        let keyed = reader
            .iter()?
            .map(|entry| {
                let (key, value) = entry?;
                Ok::<_, anyhow::Error>((
                    key.value().to_owned(),
                    V6AbstractData::from(value.into_value()),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let values = reader
            .values()?
            .map(|entry| entry.map(RecordValue::into_value).map(V6AbstractData::from))
            .collect::<Result<Vec<_>>>()?;

        assert_eq!(
            keyed
                .iter()
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            vec![first.id(), second.id()]
        );
        assert_eq!(
            keyed
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>(),
            values
        );
        Ok(())
    }

    #[test]
    fn corrupt_raw_payload_returns_contextual_decode_error() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("corrupt.redb");
        let store = DataStore::initialize_empty(&path)?;
        let transaction = store.db.begin_write()?;
        {
            let mut table = transaction.open_table(RECORDS_TABLE)?;
            table.insert("corrupt-record", &[u8::MAX][..])?;
        }
        transaction.commit()?;

        let error = store
            .reader()?
            .get("corrupt-record")
            .expect_err("corrupt payload must fail");
        assert!(error.to_string().contains("corrupt-record"));
        let error = store
            .reader()?
            .values()?
            .next()
            .expect("corrupt row")
            .expect_err("corrupt payload must fail");
        assert!(error.to_string().contains("corrupt-record"));
        Ok(())
    }

    #[test]
    fn owned_insert_validates_key_and_rolls_back_on_error() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("rollback.redb");
        let store = DataStore::initialize_empty(&path)?;
        let record = fixture("rollback-record", 8);

        let mismatch = store.write(|writer| writer.insert_v6_at("different-key", record.clone()));
        assert!(mismatch.is_err());
        assert_eq!(store.record_count()?, 0);

        let rollback = store.write(|writer| {
            writer.insert_v6_at(record.id(), record.clone())?;
            Err::<(), _>(anyhow!("abort transaction"))
        });
        assert!(rollback.is_err());
        assert_eq!(store.record_count()?, 0);
        Ok(())
    }

    #[cfg(feature = "performance-test")]
    #[test]
    #[ignore = "local V6 codec allocation microbenchmark"]
    #[allow(clippy::too_many_lines)]
    fn raw_reusable_codec_microbench_beats_typed_value_reference() -> Result<()> {
        use std::hint::black_box;

        const RECORDS: usize = 20_000;
        const SAMPLES: usize = 9;

        fn median(mut samples: Vec<Duration>) -> Duration {
            samples.sort_unstable();
            samples[samples.len() / 2]
        }

        fn legacy_insert(path: &Path, records: &[V6AbstractData]) -> Result<Duration> {
            let database = Database::create(path)?;
            let started = Instant::now();
            let mut transaction = database.begin_write()?;
            transaction.set_durability(Durability::None)?;
            {
                let definition = TableDefinition::<&str, V6AbstractData>::new("records");
                let mut table = transaction.open_table(definition)?;
                for record in records {
                    table.insert(record.id(), record.clone())?;
                }
            }
            transaction.commit()?;
            Ok(started.elapsed())
        }

        fn reusable_insert(path: &Path, records: &[V6AbstractData]) -> Result<Duration> {
            let store = DataStore::initialize_empty(path)?;
            let started = Instant::now();
            store.write_with_durability(Durability::None, |writer| {
                for record in records {
                    writer.insert_v6_at(record.id(), record.clone())?;
                }
                Ok::<(), anyhow::Error>(())
            })?;
            Ok(started.elapsed())
        }

        fn legacy_scan(path: &Path) -> Result<Duration> {
            let database = Database::open(path)?;
            let transaction = database.begin_read()?;
            let definition = TableDefinition::<&str, V6AbstractData>::new("records");
            let table = transaction.open_table(definition)?;
            let started = Instant::now();
            for entry in table.iter()? {
                let (_, value) = entry?;
                black_box(value.value().into_domain()?);
            }
            Ok(started.elapsed())
        }

        fn reusable_scan(path: &Path) -> Result<Duration> {
            let store = DataStore::open(path)?;
            let reader = store.reader()?;
            let started = Instant::now();
            for entry in reader.iter()? {
                black_box(entry?.1.into_value());
            }
            Ok(started.elapsed())
        }

        let directory = tempfile::tempdir()?;
        let records = (0..RECORDS)
            .map(|index| {
                V6AbstractData::from(AbstractData::generate_performance_data(
                    index as u64,
                    20_260_718,
                ))
            })
            .collect::<Vec<_>>();

        legacy_insert(&directory.path().join("insert-warm-legacy.redb"), &records)?;
        reusable_insert(
            &directory.path().join("insert-warm-reusable.redb"),
            &records,
        )?;

        let mut legacy_insert_samples = Vec::with_capacity(SAMPLES);
        let mut reusable_insert_samples = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            if sample.is_multiple_of(2) {
                legacy_insert_samples.push(legacy_insert(
                    &directory
                        .path()
                        .join(format!("insert-{sample}-legacy.redb")),
                    &records,
                )?);
                reusable_insert_samples.push(reusable_insert(
                    &directory
                        .path()
                        .join(format!("insert-{sample}-reusable.redb")),
                    &records,
                )?);
            } else {
                reusable_insert_samples.push(reusable_insert(
                    &directory
                        .path()
                        .join(format!("insert-{sample}-reusable.redb")),
                    &records,
                )?);
                legacy_insert_samples.push(legacy_insert(
                    &directory
                        .path()
                        .join(format!("insert-{sample}-legacy.redb")),
                    &records,
                )?);
            }
        }

        let scan_path = directory.path().join("scan.redb");
        reusable_insert(&scan_path, &records)?;
        black_box(legacy_scan(&scan_path)?);
        black_box(reusable_scan(&scan_path)?);
        let mut legacy_scan_samples = Vec::with_capacity(SAMPLES);
        let mut reusable_scan_samples = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            if sample.is_multiple_of(2) {
                legacy_scan_samples.push(legacy_scan(&scan_path)?);
                reusable_scan_samples.push(reusable_scan(&scan_path)?);
            } else {
                reusable_scan_samples.push(reusable_scan(&scan_path)?);
                legacy_scan_samples.push(legacy_scan(&scan_path)?);
            }
        }

        let legacy_insert_median = median(legacy_insert_samples);
        let reusable_insert_median = median(reusable_insert_samples);
        let legacy_scan_median = median(legacy_scan_samples);
        let reusable_scan_median = median(reusable_scan_samples);
        eprintln!(
            "V6 codec microbench: insert legacy={legacy_insert_median:?} reusable={reusable_insert_median:?}; \
             scan legacy={legacy_scan_median:?} reusable={reusable_scan_median:?}"
        );
        assert!(
            reusable_insert_median.as_secs_f64() <= legacy_insert_median.as_secs_f64() * 0.85,
            "reusable encode+insert missed the 15% target"
        );
        assert!(
            reusable_scan_median.as_secs_f64() <= legacy_scan_median.as_secs_f64() * 0.95,
            "reusable decode scan missed the 5% target"
        );
        Ok(())
    }
}
