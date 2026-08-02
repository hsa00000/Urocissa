use log::kv::Key;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct PerformanceEvent<'a> {
    schema_version: u8,
    sequence: u64,
    timestamp_ms: u128,
    phase: String,
    target: &'a str,
    level: String,
    operation: Option<String>,
    duration_ns: Option<u64>,
    message: String,
}

struct Recorder {
    sequence: AtomicU64,
    phase: Mutex<String>,
    writer: Mutex<BufWriter<File>>,
}

static RECORDER: OnceLock<Recorder> = OnceLock::new();

pub fn initialize() {
    let Some(path) = std::env::var_os("UROCISSA_PERF_EVENTS") else {
        return;
    };

    let path = std::path::PathBuf::from(path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("failed to open UROCISSA_PERF_EVENTS");

    let _ = RECORDER.set(Recorder {
        sequence: AtomicU64::new(0),
        phase: Mutex::new("startup".to_string()),
        writer: Mutex::new(BufWriter::new(file)),
    });
}

pub fn set_phase(phase: &str) {
    let Some(recorder) = RECORDER.get() else {
        return;
    };
    if let Ok(mut current) = recorder.phase.lock() {
        *current = phase.to_string();
    }
    write_event(
        recorder,
        "urocissa::performance",
        "INFO",
        Some("phase"),
        None,
        phase,
    );
}

pub fn record_log(record: &log::Record<'_>) {
    let Some(recorder) = RECORDER.get() else {
        return;
    };

    let operation = record
        .key_values()
        .get(Key::from("operation"))
        .map(|value| clean_value(&value));
    let duration_ns = record
        .key_values()
        .get(Key::from("duration_ns"))
        .and_then(|value| clean_value(&value).parse::<u64>().ok());

    if operation.is_none() && duration_ns.is_none() {
        return;
    }

    let phase = recorder
        .phase
        .lock()
        .map_or_else(|_| "unknown".to_string(), |value| value.clone());
    let sequence = recorder.sequence.fetch_add(1, Ordering::Relaxed);
    let event = PerformanceEvent {
        schema_version: 1,
        sequence,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis()),
        phase,
        target: record.target(),
        level: record.level().to_string(),
        operation,
        duration_ns,
        message: record.args().to_string(),
    };

    let Ok(line) = serde_json::to_string(&event) else {
        return;
    };
    if let Ok(mut writer) = recorder.writer.lock() {
        let _ = writeln!(writer, "{line}");
    }
}

fn write_event(
    recorder: &Recorder,
    target: &str,
    level: &str,
    operation: Option<&str>,
    duration_ns: Option<u64>,
    message: &str,
) {
    let phase = recorder
        .phase
        .lock()
        .map_or_else(|_| "unknown".to_string(), |value| value.clone());
    let sequence = recorder.sequence.fetch_add(1, Ordering::Relaxed);
    let event = PerformanceEvent {
        schema_version: 1,
        sequence,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis()),
        phase,
        target,
        level: level.to_string(),
        operation: operation.map(str::to_string),
        duration_ns,
        message: message.to_string(),
    };
    if let Ok(line) = serde_json::to_string(&event)
        && let Ok(mut writer) = recorder.writer.lock()
    {
        let _ = writeln!(writer, "{line}");
    }
}

fn clean_value(value: &log::kv::Value<'_>) -> String {
    format!("{value}").trim_matches('"').to_string()
}

pub fn flush() {
    if let Some(recorder) = RECORDER.get()
        && let Ok(mut writer) = recorder.writer.lock()
    {
        let _ = writer.flush();
    }
}
