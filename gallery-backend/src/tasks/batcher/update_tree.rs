use crate::public::db::tree::TREE;
use crate::public::db::tree::state::TreeState;
use crate::storage::store::RecordReader;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_expire::UpdateExpireTask;
use anyhow::{Context, Result};
use chrono::Utc;
use mini_executor::BatchTask;
use std::collections::HashSet;
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
    .copied()
    .collect()
});

pub struct UpdateTreeTask;

impl BatchTask for UpdateTreeTask {
    async fn batch_run(_: Vec<Self>) {
        if let Err(error) = update_tree_task() {
            error!("Failed to rebuild the in-memory tree: {error:#}");
        }
    }
}

pub fn update_tree_task() -> Result<(usize, usize)> {
    let data_table = TREE
        .store
        .reader()
        .context("failed to open the V6 records table for tree rebuild")?;
    update_tree_from_reader(&data_table)
}

pub fn update_tree_from_reader(data_table: &RecordReader) -> Result<(usize, usize)> {
    let start_time = Instant::now();
    let counts = TREE.with_list_snapshot_update(|| {
        let state = tree_state_from_reader(data_table)?;
        let counts = (state.len(), state.albums.len());
        TREE.replace_tree_snapshot(
            state,
            crate::public::db::tree::read_tags::TreeListSnapshot::default(),
        );
        Ok::<_, anyhow::Error>(counts)
    })?;

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = Utc::now().timestamp_millis();
    crate::perf_timing!(
        "tree.rebuild",
        start_time,
        "In-memory cache updated ({}).",
        current_timestamp
    );
    Ok(counts)
}

fn tree_state_from_reader(data_table: &RecordReader) -> Result<TreeState> {
    let records = data_table.iter()?.map(|guard| {
        let (_, value) = guard?;
        let mut abstract_data = value.into_value();
        // Retain only EXIF fields that are used by query search.
        if let Some(exif_vec) = abstract_data.exif_vec_mut() {
            exif_vec.retain(|key, _| ALLOWED_KEYS.contains(&key.as_str()));
        }
        Ok::<_, anyhow::Error>(abstract_data)
    });
    TreeState::try_from_records(records)
}

#[cfg(test)]
mod tests {
    use redb::{Database, TableDefinition, TypeName, Value};
    use tempfile::tempdir;

    use super::*;
    use crate::storage::DataStore;

    #[derive(Debug)]
    struct RawV6Bytes;

    impl Value for RawV6Bytes {
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
            TypeName::new("AbstractDataV6")
        }
    }

    #[test]
    fn startup_scan_stops_at_corrupt_v6_record_and_reports_key() -> Result<()> {
        let directory = tempdir()?;
        let path = directory.path().join("index_v6.redb");
        let database = Database::create(&path)?;
        let transaction = database.begin_write()?;
        {
            let mut table =
                transaction.open_table(TableDefinition::<&str, RawV6Bytes>::new("records"))?;
            table.insert("broken-record", b"not-valid-bitcode".as_slice())?;
        }
        transaction.commit()?;
        drop(database);

        let store = DataStore::open(&path)?;
        let reader = store.reader()?;
        let error = tree_state_from_reader(&reader).unwrap_err();
        assert!(error.to_string().contains("broken-record"));
        Ok(())
    }
}
