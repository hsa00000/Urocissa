use arrayvec::ArrayString;
use dashmap::DashSet;
use std::sync::LazyLock;

// ────────────────────────────────────────────────────────────────
// ProcessingGuard - Prevents duplicate processing of the same hash
// ────────────────────────────────────────────────────────────────

static IN_PROGRESS: LazyLock<DashSet<ArrayString<64>>> = LazyLock::new(DashSet::new);

pub struct ProcessingGuard(ArrayString<64>);

impl Drop for ProcessingGuard {
    fn drop(&mut self) {
        IN_PROGRESS.remove(&self.0);
    }
}

pub fn try_acquire(hash: impl AsRef<str>) -> Option<ProcessingGuard> {
    let hash = ArrayString::from(hash.as_ref()).ok()?;
    if IN_PROGRESS.insert(hash) {
        Some(ProcessingGuard(hash))
    } else {
        None
    }
}
