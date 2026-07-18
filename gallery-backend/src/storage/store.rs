use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};

use anyhow::{Context, Result, anyhow};
use redb::{
    Database, Durability, ReadOnlyTable, ReadableDatabase, ReadableTable, ReadableTableMetadata,
    Table, TableDefinition,
};

use super::v6::V6AbstractData;
use crate::public::structure::abstract_data::AbstractData;

pub const RECORDS_TABLE: TableDefinition<&str, V6AbstractData> = TableDefinition::new("records");

pub struct DataStore {
    db: Database,
}

pub struct RecordReader {
    table: ReadOnlyTable<&'static str, V6AbstractData>,
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
    table: Table<'txn, &'static str, V6AbstractData>,
}

impl DataStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create database directory {}", parent.display())
            })?;
        }
        Ok(Self {
            db: Database::create(path)
                .with_context(|| format!("failed to open V6 database {}", path.display()))?,
        })
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
        self.read(|reader| reader.len())
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
        let mut writer = RecordWriter { table };
        let result = operation(&mut writer)?;
        drop(writer);
        txn.commit().map_err(anyhow::Error::from).map_err(E::from)?;
        Ok(result)
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
        let value = catch_unwind(AssertUnwindSafe(|| value.value()))
            .map_err(|_| anyhow!("failed to decode V6 record {key}: bitcode payload is invalid"))?
            .into_domain()
            .with_context(|| format!("failed to convert V6 record {key}"))?;
        Ok(Some(RecordValue(value)))
    }

    #[allow(clippy::iter_not_returning_iterator)]
    pub fn iter(&self) -> Result<impl Iterator<Item = Result<(RecordKey, RecordValue)>> + '_> {
        Ok(self.table.iter()?.map(|entry| {
            let (key, value) = entry?;
            let key = key.value().to_owned();
            let value = catch_unwind(AssertUnwindSafe(|| value.value()))
                .map_err(|_| {
                    anyhow!("failed to decode V6 record {key}: bitcode payload is invalid")
                })?
                .into_domain()
                .with_context(|| format!("failed to convert V6 record {key}"))?;
            Ok((RecordKey(key), RecordValue(value)))
        }))
    }
}

impl RecordWriter<'_> {
    pub fn get(&self, key: &str) -> Result<Option<RecordValue>> {
        let Some(value) = self.table.get(key)? else {
            return Ok(None);
        };
        let value = catch_unwind(AssertUnwindSafe(|| value.value()))
            .map_err(|_| anyhow!("failed to decode V6 record {key}: bitcode payload is invalid"))?
            .into_domain()
            .with_context(|| format!("failed to convert V6 record {key}"))?;
        Ok(Some(RecordValue(value)))
    }

    pub fn get_v6(&self, key: &str) -> Result<Option<V6AbstractData>> {
        let Some(value) = self.table.get(key)? else {
            return Ok(None);
        };
        let value = catch_unwind(AssertUnwindSafe(|| value.value()))
            .map_err(|_| anyhow!("failed to decode V6 record {key}: bitcode payload is invalid"))?;
        Ok(Some(value))
    }

    pub fn insert(&mut self, value: &AbstractData) -> Result<()> {
        let key = value.hash();
        let stored = V6AbstractData::from(value);
        self.table.insert(key.as_str(), stored)?;
        Ok(())
    }

    pub fn insert_at(&mut self, key: &str, value: &AbstractData) -> Result<()> {
        if value.hash().as_str() != key {
            return Err(anyhow!(
                "record key {key} does not match record id {}",
                value.hash()
            ));
        }
        self.insert_v6_at(key, &V6AbstractData::from(value))
    }

    pub fn insert_v6_at(&mut self, key: &str, value: &V6AbstractData) -> Result<()> {
        if value.id() != key {
            return Err(anyhow!(
                "record key {key} does not match V6 record id {}",
                value.id()
            ));
        }
        self.table.insert(key, value.clone())?;
        Ok(())
    }

    pub fn remove(&mut self, key: &str) -> Result<()> {
        self.table.remove(key)?;
        Ok(())
    }
}
