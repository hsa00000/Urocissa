use std::{
    collections::{BTreeMap, HashSet},
    hint::black_box,
    path::Path,
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use arrayvec::ArrayString;
use redb::{Durability, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::Serialize;

use crate::{
    performance,
    public::db::tree::state::TreeState,
    storage::{
        DataStore,
        cache::{CacheClass, MIGRATION_CACHE_BYTES, database_builder},
        legacy_v5::{
            LegacyAbstractData, LegacyFileModify, LegacyImageCombined, LegacyImageMetadata,
            LegacyObjectSchema, LegacyObjectType, LegacyVideoCombined, LegacyVideoMetadata,
        },
        migration::prepare_storage_at,
        v6::V6AbstractData,
    },
};

const LEGACY_TABLE: TableDefinition<&str, LegacyAbstractData> = TableDefinition::new("database");
const FIXTURE_BATCH_SIZE: u64 = 16_384;
const DEFAULT_RECORDS: u64 = 1_000_000;
const DEFAULT_SAMPLES: usize = 3;
const RSS_LIMIT_BYTES: u64 = 850 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheMetrics {
    limit_bytes: usize,
    used_bytes: usize,
    evictions: u64,
    read_hits: u64,
    read_misses: u64,
    write_hits: u64,
    write_misses: u64,
}

impl CacheMetrics {
    fn new(limit_bytes: usize, stats: &redb::CacheStats) -> Self {
        Self {
            limit_bytes,
            used_bytes: stats.used_bytes(),
            evictions: stats.evictions(),
            read_hits: stats.read_hits(),
            read_misses: stats.read_misses(),
            write_hits: stats.write_hits(),
            write_misses: stats.write_misses(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FormatSample {
    storage_open_ms: f64,
    record_count_ms: f64,
    decode_scan_ms: f64,
    tree_state_with_decode_ms: f64,
    tree_state_build_estimate_ms: f64,
    records_per_second: f64,
    peak_rss_bytes: u64,
    cache: CacheMetrics,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GateSummary {
    v5_median_ms: f64,
    v6_median_ms: f64,
    v6_to_v5_ratio: f64,
    maximum_peak_rss_bytes: u64,
    relative_speed_passed: bool,
    rss_passed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageHarnessReport {
    schema_version: u32,
    records: u64,
    samples: usize,
    fixture_generation_ms: f64,
    migration_wall_ms: f64,
    migration_records_per_second: f64,
    migration_peak_rss_bytes: u64,
    migration_source_record_scans: u8,
    migration_destination_record_scans: u8,
    normal_startup_record_iterations: u8,
    v5: Vec<FormatSample>,
    v6: Vec<FormatSample>,
    gate: GateSummary,
}

pub fn requested() -> bool {
    std::env::var_os("UROCISSA_STORAGE_HARNESS").is_some()
}

pub fn run() -> Result<()> {
    let root = crate::public::constant::storage::get_data_path();
    let marker = root.join(".urocissa-performance-root");
    if !marker.is_file() {
        bail!(
            "storage harness refuses to use {} without {}",
            root.display(),
            marker.display()
        );
    }

    let records = read_env("UROCISSA_STORAGE_COUNT", DEFAULT_RECORDS)?;
    let samples = read_env("UROCISSA_STORAGE_SAMPLES", DEFAULT_SAMPLES)?;
    if records == 0 {
        bail!("UROCISSA_STORAGE_COUNT must be greater than zero");
    }
    if samples == 0 {
        bail!("UROCISSA_STORAGE_SAMPLES must be greater than zero");
    }

    let db_dir = root.join("db");
    std::fs::create_dir_all(&db_dir)?;
    let v5_path = db_dir.join("index_v5.redb");
    let v6_path = db_dir.join("index_v6.redb");

    let fixture_started = Instant::now();
    create_v5_fixture(&v5_path, records)?;
    let fixture_generation_ms = elapsed_ms(fixture_started);

    let mut v5 = Vec::with_capacity(samples);
    for sample in 0..samples {
        v5.push(benchmark_v5(&v5_path, records, sample)?);
    }

    performance::set_phase("storage-harness.migration");
    let migration_started = Instant::now();
    prepare_storage_at(&db_dir)?;
    let migration_wall_ms = elapsed_ms(migration_started);
    let migration_peak_rss_bytes = performance::memory_snapshot().phase_peak_rss_bytes;

    let mut v6 = Vec::with_capacity(samples);
    for sample in 0..samples {
        v6.push(benchmark_v6(&v6_path, records, sample)?);
    }

    let v5_median_ms = median(
        v5.iter()
            .map(|sample| sample.tree_state_with_decode_ms)
            .collect(),
    );
    let v6_median_ms = median(
        v6.iter()
            .map(|sample| sample.tree_state_with_decode_ms)
            .collect(),
    );
    let v6_to_v5_ratio = v6_median_ms / v5_median_ms.max(f64::EPSILON);
    let maximum_peak_rss_bytes = v5
        .iter()
        .chain(&v6)
        .map(|sample| sample.peak_rss_bytes)
        .chain([migration_peak_rss_bytes])
        .max()
        .unwrap_or(0);
    let gate = GateSummary {
        v5_median_ms,
        v6_median_ms,
        v6_to_v5_ratio,
        maximum_peak_rss_bytes,
        relative_speed_passed: v6_to_v5_ratio <= 1.15,
        rss_passed: maximum_peak_rss_bytes <= RSS_LIMIT_BYTES,
    };
    let report = StorageHarnessReport {
        schema_version: 1,
        records,
        samples,
        fixture_generation_ms,
        migration_wall_ms,
        migration_records_per_second: records_per_second(records, migration_wall_ms),
        migration_peak_rss_bytes,
        migration_source_record_scans: 1,
        migration_destination_record_scans: 0,
        normal_startup_record_iterations: 1,
        v5,
        v6,
        gate,
    };

    let output = std::env::var_os("UROCISSA_STORAGE_RESULT")
        .map_or_else(|| root.join("storage-harness.json"), Into::into);
    std::fs::write(&output, serde_json::to_vec_pretty(&report)?).with_context(|| {
        format!(
            "failed to write storage harness result {}",
            output.display()
        )
    })?;
    println!("Storage harness result: {}", output.display());

    if !report.gate.relative_speed_passed || !report.gate.rss_passed {
        bail!(
            "storage gate failed: V6/V5={:.3} (limit 1.150), peak RSS={} bytes (limit {} bytes)",
            report.gate.v6_to_v5_ratio,
            report.gate.maximum_peak_rss_bytes,
            RSS_LIMIT_BYTES
        );
    }
    Ok(())
}

fn create_v5_fixture(path: &Path, records: u64) -> Result<()> {
    let database = database_builder(CacheClass::Migration).create(path)?;
    let mut start = 0_u64;
    while start < records {
        let end = (start + FIXTURE_BATCH_SIZE).min(records);
        let mut transaction = database.begin_write()?;
        transaction.set_durability(Durability::None)?;
        {
            let mut table = transaction.open_table(LEGACY_TABLE)?;
            for index in start..end {
                let (id, value) = legacy_fixture(index)?;
                table.insert(id.as_str(), value)?;
            }
        }
        transaction.commit()?;
        start = end;
    }

    let mut transaction = database.begin_write()?;
    transaction.set_durability(Durability::Immediate)?;
    {
        let _table = transaction.open_table(LEGACY_TABLE)?;
    }
    transaction.commit()?;
    Ok(())
}

fn benchmark_v5(path: &Path, expected: u64, sample: usize) -> Result<FormatSample> {
    performance::set_phase(&format!("storage-harness.v5.{sample}"));
    let open_started = Instant::now();
    let database = database_builder(CacheClass::Migration).open_read_only(path)?;
    let transaction = database.begin_read()?;
    let table = transaction.open_table(LEGACY_TABLE)?;
    let storage_open_ms = elapsed_ms(open_started);

    let count_started = Instant::now();
    let count = table.len()?;
    let record_count_ms = elapsed_ms(count_started);
    ensure_count("V5", count, expected)?;

    let decode_started = Instant::now();
    for entry in table.iter()? {
        let (_, value) = entry?;
        let domain = V6AbstractData::from_v5(value.value())?.into_domain()?;
        black_box(domain.cache_version());
    }
    let decode_scan_ms = elapsed_ms(decode_started);

    let build_started = Instant::now();
    let records = table.iter()?.map(|entry| {
        let (_, value) = entry?;
        V6AbstractData::from_v5(value.value())?.into_domain()
    });
    let state = TreeState::try_from_records_with_capacity(
        records,
        usize::try_from(expected).context("V5 fixture count exceeds usize")?,
    )?;
    ensure_count("V5 TreeState", state.len() as u64, expected)?;
    let tree_state_with_decode_ms = elapsed_ms(build_started);
    let peak_rss_bytes = performance::memory_snapshot().phase_peak_rss_bytes;
    black_box(&state);
    drop(state);

    Ok(FormatSample {
        storage_open_ms,
        record_count_ms,
        decode_scan_ms,
        tree_state_with_decode_ms,
        tree_state_build_estimate_ms: (tree_state_with_decode_ms - decode_scan_ms).max(0.0),
        records_per_second: records_per_second(expected, tree_state_with_decode_ms),
        peak_rss_bytes,
        cache: CacheMetrics::new(MIGRATION_CACHE_BYTES, &database.cache_stats()),
    })
}

fn benchmark_v6(path: &Path, expected: u64, sample: usize) -> Result<FormatSample> {
    performance::set_phase(&format!("storage-harness.v6.{sample}"));
    let open_started = Instant::now();
    let store = DataStore::open(path)?;
    let reader = store.reader()?;
    let storage_open_ms = elapsed_ms(open_started);

    let count_started = Instant::now();
    let count = reader.len()?;
    let record_count_ms = elapsed_ms(count_started);
    ensure_count("V6", count, expected)?;

    let decode_started = Instant::now();
    for entry in reader.values()? {
        let value = entry?;
        black_box(value.into_value().cache_version());
    }
    let decode_scan_ms = elapsed_ms(decode_started);

    let build_started = Instant::now();
    let records = reader
        .values()?
        .map(|entry| entry.map(crate::storage::store::RecordValue::into_value));
    let state = TreeState::try_from_records_with_capacity(
        records,
        usize::try_from(expected).context("V6 fixture count exceeds usize")?,
    )?;
    ensure_count("V6 TreeState", state.len() as u64, expected)?;
    let tree_state_with_decode_ms = elapsed_ms(build_started);
    let peak_rss_bytes = performance::memory_snapshot().phase_peak_rss_bytes;
    black_box(&state);
    drop(state);

    Ok(FormatSample {
        storage_open_ms,
        record_count_ms,
        decode_scan_ms,
        tree_state_with_decode_ms,
        tree_state_build_estimate_ms: (tree_state_with_decode_ms - decode_scan_ms).max(0.0),
        records_per_second: records_per_second(expected, tree_state_with_decode_ms),
        peak_rss_bytes,
        cache: CacheMetrics::new(store.cache_limit_bytes(), &store.cache_stats()),
    })
}

fn legacy_fixture(index: u64) -> Result<(ArrayString<64>, LegacyAbstractData)> {
    let id = ArrayString::<64>::from(&format!("{index:016x}"))
        .map_err(|_| anyhow!("fixture id exceeded ArrayString capacity"))?;
    let object_type = if index.is_multiple_of(20) {
        LegacyObjectType::Video
    } else {
        LegacyObjectType::Image
    };
    let object = LegacyObjectSchema {
        id,
        obj_type: object_type,
        pending: false,
        thumbhash: Some(vec![1, 2, 3, (index & 0xff) as u8]),
        description: index
            .is_multiple_of(10)
            .then(|| format!("fixture record {index}")),
        tags: HashSet::from([format!("fixture-tag-{}", index % 64)]),
        is_favorite: index.is_multiple_of(7),
        is_archived: index.is_multiple_of(31),
        is_trashed: index.is_multiple_of(97),
        update_at: i64::try_from(index).unwrap_or(i64::MAX),
    };
    let alias = vec![LegacyFileModify {
        file: format!("{id}.jpg"),
        modified: i64::try_from(index).unwrap_or(i64::MAX),
        scan_time: i64::try_from(index.saturating_add(1)).unwrap_or(i64::MAX),
    }];
    let exif_vec = BTreeMap::from([("Make".to_owned(), "Urocissa fixture".to_owned())]);

    let value = if matches!(object_type, LegacyObjectType::Video) {
        LegacyAbstractData::Video(LegacyVideoCombined {
            object,
            metadata: LegacyVideoMetadata {
                id,
                size: 2_000_000 + index,
                width: 1_920,
                height: 1_080,
                ext: "mp4".to_owned(),
                duration: 10.0 + f64::from(u32::try_from(index % 300).unwrap()),
                albums: HashSet::new(),
                exif_vec,
                alias,
            },
        })
    } else {
        LegacyAbstractData::Image(LegacyImageCombined {
            object,
            metadata: LegacyImageMetadata {
                id,
                size: 1_000_000 + index,
                width: 4_032,
                height: 3_024,
                ext: "jpg".to_owned(),
                phash: Some(vec![0; 8]),
                albums: HashSet::new(),
                exif_vec,
                alias,
            },
        })
    };
    Ok((id, value))
}

fn ensure_count(label: &str, actual: u64, expected: u64) -> Result<()> {
    if actual != expected {
        bail!("{label} count mismatch: {actual}, expected {expected}");
    }
    Ok(())
}

fn read_env<T>(name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Some(value) = std::env::var_os(name) else {
        return Ok(default);
    };
    value
        .to_string_lossy()
        .parse()
        .map_err(|error| anyhow!("invalid {name}: {error}"))
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1_000.0
}

#[allow(clippy::cast_precision_loss)]
fn records_per_second(records: u64, elapsed_ms: f64) -> f64 {
    records as f64 / (elapsed_ms / 1_000.0)
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        f64::midpoint(values[middle - 1], values[middle])
    } else {
        values[middle]
    }
}
