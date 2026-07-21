use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct MemorySnapshot {
    pub current_rss_bytes: u64,
    pub global_peak_rss_bytes: u64,
    pub phase_peak_rss_bytes: u64,
    pub phase_average_rss_bytes: u64,
    pub phase_sample_count: u64,
}

#[derive(Debug, Default)]
struct PeakTracker {
    current_rss_bytes: u64,
    global_peak_rss_bytes: u64,
    phase_peak_rss_bytes: u64,
    phase_rss_sum: u128,
    phase_sample_count: u64,
}

impl PeakTracker {
    fn observe(&mut self, rss_bytes: u64) {
        self.current_rss_bytes = rss_bytes;
        self.global_peak_rss_bytes = self.global_peak_rss_bytes.max(rss_bytes);
        self.phase_peak_rss_bytes = self.phase_peak_rss_bytes.max(rss_bytes);
        self.phase_rss_sum = self.phase_rss_sum.saturating_add(u128::from(rss_bytes));
        self.phase_sample_count = self.phase_sample_count.saturating_add(1);
    }

    fn reset_phase(&mut self) {
        self.phase_peak_rss_bytes = self.current_rss_bytes;
        self.phase_rss_sum = u128::from(self.current_rss_bytes);
        self.phase_sample_count = 1;
    }

    fn snapshot(&self) -> MemorySnapshot {
        let phase_average_rss_bytes = if self.phase_sample_count == 0 {
            0
        } else {
            u64::try_from(self.phase_rss_sum / u128::from(self.phase_sample_count))
                .unwrap_or(u64::MAX)
        };
        MemorySnapshot {
            current_rss_bytes: self.current_rss_bytes,
            global_peak_rss_bytes: self.global_peak_rss_bytes,
            phase_peak_rss_bytes: self.phase_peak_rss_bytes,
            phase_average_rss_bytes,
            phase_sample_count: self.phase_sample_count,
        }
    }
}

struct Sampler {
    system: sysinfo::System,
    pid: sysinfo::Pid,
    peaks: PeakTracker,
}

impl Sampler {
    fn new() -> Self {
        Self {
            system: sysinfo::System::new(),
            pid: sysinfo::Pid::from_u32(std::process::id()),
            peaks: PeakTracker::default(),
        }
    }

    fn sample(&mut self) {
        self.system
            .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.pid]), true);
        let rss_bytes = self
            .system
            .process(self.pid)
            .map_or(0, sysinfo::Process::memory);
        self.peaks.observe(rss_bytes);
    }
}

static SAMPLER: OnceLock<Mutex<Sampler>> = OnceLock::new();
static SAMPLER_THREAD: OnceLock<()> = OnceLock::new();

fn sampler() -> &'static Mutex<Sampler> {
    SAMPLER.get_or_init(|| Mutex::new(Sampler::new()))
}

pub fn initialize() {
    if let Ok(mut state) = sampler().lock() {
        state.sample();
    }
    SAMPLER_THREAD.get_or_init(|| {
        std::thread::Builder::new()
            .name("rss-sampler".to_string())
            .spawn(|| {
                loop {
                    std::thread::sleep(Duration::from_millis(100));
                    if let Ok(mut state) = sampler().lock() {
                        state.sample();
                    }
                }
            })
            .expect("failed to start RSS sampler");
    });
}

pub fn set_phase() {
    if let Ok(mut state) = sampler().lock() {
        state.sample();
        state.peaks.reset_phase();
    }
}

pub fn memory_snapshot() -> MemorySnapshot {
    sampler().lock().map_or_else(
        |_| MemorySnapshot::default(),
        |mut state| {
            state.sample();
            state.peaks.snapshot()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn peaks_are_monotonic_and_phase_peak_resets_to_current() {
        let mut tracker = PeakTracker::default();
        tracker.observe(100);
        tracker.observe(80);
        tracker.reset_phase();
        tracker.observe(90);
        assert_eq!(
            tracker.snapshot(),
            MemorySnapshot {
                current_rss_bytes: 90,
                global_peak_rss_bytes: 100,
                phase_peak_rss_bytes: 90,
                phase_average_rss_bytes: 85,
                phase_sample_count: 2,
            }
        );
    }

    #[test]
    fn mutex_serializes_concurrent_peak_updates() {
        let tracker = Arc::new(Mutex::new(PeakTracker::default()));
        let handles = (0..8)
            .map(|worker| {
                let tracker = Arc::clone(&tracker);
                std::thread::spawn(move || {
                    for value in 0..1_000 {
                        tracker.lock().unwrap().observe(worker * 1_000 + value);
                    }
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(
            tracker.lock().unwrap().snapshot().global_peak_rss_bytes,
            7_999
        );
    }
}
