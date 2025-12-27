use crate::public::tui::DASHBOARD;

pub struct PendingGuard;

impl PendingGuard {
    pub fn new() -> Self {
        DASHBOARD.increase_pending();
        Self
    }
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        DASHBOARD.decrease_pending();
    }
}
