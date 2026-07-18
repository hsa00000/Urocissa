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

#[cfg(feature = "performance-test")]
mod recorder;

#[cfg(feature = "performance-test")]
pub use recorder::{flush, initialize, record_log, set_phase};

#[cfg(not(feature = "performance-test"))]
pub fn initialize() {}

#[cfg(not(feature = "performance-test"))]
pub fn record_log(_record: &log::Record<'_>) {}

#[cfg(not(feature = "performance-test"))]
#[allow(dead_code)]
pub fn set_phase(_phase: &str) {}

#[cfg(not(feature = "performance-test"))]
#[allow(dead_code)]
pub fn flush() {}
