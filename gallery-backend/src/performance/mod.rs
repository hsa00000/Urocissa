//! Performance-test instrumentation and feature-gated control helpers.

/// Emit a human-readable TUI timing while attaching stable machine-readable
/// operation metadata for the performance recorder.
#[macro_export]
macro_rules! perf_timing {
    ($operation:expr, $start:expr, $($arg:tt)*) => {{
        let elapsed = $start.elapsed();
        let duration = format!("{:?}", elapsed);
        let duration_ns = elapsed.as_nanos().min(u64::MAX as u128) as u64;
        log::info!(
            operation = $operation,
            duration_ns = duration_ns,
            duration = &*duration;
            $($arg)*
        );
    }};
}

/// Emit a pre-aggregated performance duration without taking another
/// `Instant`. This is used by opt-in detailed profiling where the measured
/// sub-operations are accumulated inside a transaction.
#[macro_export]
macro_rules! perf_duration {
    ($operation:expr, $elapsed:expr, $($arg:tt)*) => {{
        let elapsed = $elapsed;
        let duration = format!("{:?}", elapsed);
        let duration_ns = elapsed.as_nanos().min(u64::MAX as u128) as u64;
        log::info!(
            operation = $operation,
            duration_ns = duration_ns,
            duration = &*duration;
            $($arg)*
        );
    }};
}

#[cfg(feature = "performance-test")]
mod memory;

#[cfg(feature = "performance-test")]
mod recorder;

#[cfg(feature = "performance-test")]
mod storage_harness;

#[cfg(feature = "performance-test")]
pub use memory::memory_snapshot;

#[cfg(feature = "performance-test")]
pub use recorder::{flush, record_log};

#[cfg(feature = "performance-test")]
pub use storage_harness::{requested as storage_harness_requested, run as run_storage_harness};

#[cfg(feature = "performance-test")]
pub fn initialize() {
    recorder::initialize();
    memory::initialize();
}

#[cfg(feature = "performance-test")]
pub fn set_phase(phase: &str) {
    recorder::set_phase(phase);
    memory::set_phase();
}

#[cfg(feature = "performance-test")]
pub fn detailed_timing_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("UROCISSA_PERF_DETAILED_TIMING").is_ok_and(|value| value == "1")
    })
}

#[cfg(not(feature = "performance-test"))]
pub fn initialize() {}

#[cfg(not(feature = "performance-test"))]
pub fn record_log(_record: &log::Record<'_>) {}

#[cfg(not(feature = "performance-test"))]
#[allow(dead_code)]
pub fn set_phase(_phase: &str) {}

#[cfg(not(feature = "performance-test"))]
pub const fn detailed_timing_enabled() -> bool {
    false
}

#[cfg(not(feature = "performance-test"))]
#[allow(dead_code)]
pub fn flush() {}
