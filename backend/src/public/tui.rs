//! tui.rs — lock-free dashboard (Rust 1.88)

use anyhow::Result;
use arrayvec::ArrayString;
use atomic_float::AtomicF64;
use crossbeam_queue::ArrayQueue;
use dashmap::DashMap;
use std::{
    mem,
    sync::{
        Arc, LazyLock, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use superconsole::{Component, Dimensions, DrawMode, Line, Lines};
use terminal_size::{Width, terminal_size};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::public::constant::runtime::CURRENT_NUM_THREADS;

/// ---------- async driver ----------
pub async fn tui_task(
    mut sc: superconsole::SuperConsole,
    dashboard: std::sync::Arc<Dashboard>,
    mut rx: UnboundedReceiver<String>,
) -> Result<()> {
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(50));

    loop {
        tokio::select! {
            Some(line) = rx.recv() => {
                let colored = Lines::from_colored_multiline_string(&line);
                sc.emit(colored);
            }
            _ = tick.tick() => {
                sc.render(&*dashboard)?;
            }
        }
    }
}

/// ---------- task model ----------
#[derive(Debug, Clone)]
pub enum FileType {
    Image,
    Video,
}

impl TryFrom<&str> for FileType {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> Result<Self> {
        match s {
            "image" => Ok(FileType::Image),
            "video" => Ok(FileType::Video),
            _ => Err(anyhow::anyhow!("Unknown file type: {s}")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskState {
    Indexing(Instant),
    Transcoding(Instant),
    Done(f64),
    Failed(f64),
}

#[derive(Clone)]
pub struct TaskRow {
    pub hash: ArrayString<64>,
    pub path: String,
    pub file_type: FileType,
    pub state: TaskState,
    pub progress: Option<f64>,
}

impl TaskRow {
    pub fn advance_state(&mut self) {
        let old = mem::replace(&mut self.state, TaskState::Done(0.0));
        self.state = match old {
            TaskState::Indexing(t0) => match self.file_type {
                FileType::Image => TaskState::Done(t0.elapsed().as_secs_f64()),
                FileType::Video => TaskState::Transcoding(Instant::now()),
            },
            TaskState::Transcoding(t0) => TaskState::Done(t0.elapsed().as_secs_f64()),
            TaskState::Done(d) => TaskState::Done(d),
            TaskState::Failed(d) => TaskState::Failed(d),
        };
        if matches!(self.state, TaskState::Transcoding(_)) {
            self.progress = None;
        }
    }

    pub fn fmt(&self) -> String {
        const COL_STATUS: usize = 3; // Status column fixed to 3 characters
        const COL_HASH: usize = 5;
        const DEFAULT_COLS: usize = 120;

        // Generate status content
        let status = match (&self.state, self.progress) {
            // With progress: limit to 1–99, round to integer
            (TaskState::Transcoding(_), Some(p)) => {
                let pct = (p.clamp(1.0, 99.0)).round() as u8;
                format!("{:>2}%", pct) // e.g. " 1%", "99%"
            }
            // Completed
            (TaskState::Done(_), _) => " v ".to_string(),
            (TaskState::Failed(_), _) => " x ".to_string(),
            // Not started/Indexing
            _ => "   ".to_string(),
        };
        let status_col = format!("{:<COL_STATUS$}", status); // Pad to 3 characters

        // Take the first 5 characters of the hash, right-aligned
        let short_hash = &self.hash.as_str()[..COL_HASH.min(self.hash.len())];
        let hash_col = format!("{:>COL_HASH$}", short_hash);

        // Calculate elapsed seconds
        let secs = match self.state {
            TaskState::Indexing(t0) | TaskState::Transcoding(t0) => t0.elapsed().as_secs_f64(),
            TaskState::Done(d) | TaskState::Failed(d) => d,
        };
        let suffix = format!(" │ {:>6.1}s", secs);

        // Get terminal width and margin
        let margin = std::env::var("UROCISSA_TERM_MARGIN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);
        let cols = terminal_size()
            .map(|(Width(w), _)| w as usize)
            .unwrap_or(DEFAULT_COLS);

        // Calculate path column width and truncate
        let prefix_w = COL_STATUS + 3 + COL_HASH + 3;
        let path_budget = cols
            .saturating_sub(prefix_w + UnicodeWidthStr::width(suffix.as_str()) + margin)
            .max(5);
        let short_path = Self::tail_ellipsis(&self.path, path_budget);

        let pad =
            " ".repeat(path_budget.saturating_sub(UnicodeWidthStr::width(short_path.as_str())));

        format!("{status_col} │ {hash_col} │ {short_path}{pad}{suffix}")
    }

    // Truncate string to fit within max width by adding ellipsis at the front if needed
    fn tail_ellipsis(s: &str, max: usize) -> String {
        if UnicodeWidthStr::width(s) <= max {
            return s.to_owned(); // No truncation needed
        }

        let tail_len = max.saturating_sub(1); // Leave space for "…"
        let mut acc_width = 0;
        let mut rev_chars = String::new();

        // Collect characters from the end until width limit
        for c in s.chars().rev() {
            let char_width = c.width().unwrap_or(0);
            if acc_width + char_width > tail_len {
                if acc_width + 1 <= tail_len {
                    rev_chars.push('…'); // Optionally add ellipsis
                }
                break;
            }
            acc_width += char_width;
            rev_chars.push(c);
        }

        // Reverse back and prepend ellipsis
        format!("…{}", rev_chars.chars().rev().collect::<String>())
    }
}

/// ---------- dashboard ----------
pub struct Dashboard {
    tasks: DashMap<ArrayString<64>, TaskRow>,
    completed: ArrayQueue<TaskRow>,
    handled: AtomicU64,
    pending: AtomicU64,
    total_duration: AtomicF64,
}

pub static LOGGER_TX: OnceLock<UnboundedSender<String>> = OnceLock::new();
pub static DASHBOARD: LazyLock<Arc<Dashboard>> = LazyLock::new(|| Arc::new(Dashboard::new()));

impl Dashboard {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            completed: ArrayQueue::new(*CURRENT_NUM_THREADS * 4),
            handled: AtomicU64::new(0),
            pending: AtomicU64::new(0),
            total_duration: AtomicF64::new(0.0),
        }
    }

    /* ---------- mutation API ---------- */
    pub fn add_task(&self, hash: ArrayString<64>, path: String, file_type: FileType) {
        self.tasks
            .entry(hash.clone())
            .and_modify(|t| {
                t.path = path.clone();
                t.file_type = file_type.clone();
                t.state = TaskState::Indexing(Instant::now());
                t.progress = None;
            })
            .or_insert_with(|| TaskRow {
                hash,
                path,
                file_type,
                state: TaskState::Indexing(Instant::now()),
                progress: None,
            });
    }

    /// Success: advance state; if it reaches Done, remove from `running`, put into `completed`, and update statistics
    pub fn advance_task_state(&self, hash: &ArrayString<64>) {
        if let Some(mut view) = self.tasks.get_mut(hash) {
            view.advance_state();
            if let TaskState::Done(duration) = view.state {
                let row_done = view.clone();
                drop(view);
                self.tasks.remove(hash);
                self.move_to_completed(row_done, duration);
            }
        }
    }

    /// Failure: mark the current row as Failed(elapsed) and move to `completed`
    /// (does not update `handled` / `total_duration`)
    pub fn mark_failed(&self, hash: &ArrayString<64>) {
        if let Some(mut view) = self.tasks.get_mut(hash) {
            let elapsed = match view.state {
                TaskState::Indexing(t0) | TaskState::Transcoding(t0) => t0.elapsed().as_secs_f64(),
                TaskState::Done(d) | TaskState::Failed(d) => d,
            };
            view.state = TaskState::Failed(elapsed);
            let row_failed = view.clone();
            drop(view);
            self.tasks.remove(hash);
            self.push_to_completed(row_failed);
        }
    }

    /// Push into the ring buffer `completed`; if full, pop the oldest first
    fn push_to_completed(&self, row: TaskRow) {
        if let Err(r) = self.completed.push(row) {
            let _ = self.completed.pop();
            let _ = self.completed.push(r);
        }
    }

    /// Move to `completed` and update statistics
    fn move_to_completed(&self, row: TaskRow, duration: f64) {
        self.total_duration.fetch_add(duration, Ordering::Relaxed);
        self.push_to_completed(row);
        self.handled.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_progress(&self, hash: ArrayString<64>, percent: f64) {
        if let Some(mut view) = self.tasks.get_mut(&hash) {
            view.progress = Some(percent.clamp(0.0, 100.0));
        }
    }
    pub fn increase_pending(&self) {
        self.pending.fetch_add(1, Ordering::Relaxed);
    }
    pub fn decrease_pending(&self) {
        self.pending.fetch_sub(1, Ordering::Relaxed);
    }

    #[inline]
    fn handled(&self) -> u64 {
        self.handled.load(Ordering::Relaxed)
    }
    #[inline]
    fn pending(&self) -> u64 {
        self.pending.load(Ordering::Relaxed)
    }
    #[inline]
    fn total_duration(&self) -> f64 {
        self.total_duration.load(Ordering::Relaxed)
    }
}

/// ---------- renderer ----------
impl Component for Dashboard {
    fn draw_unchecked(&self, _: Dimensions, _: DrawMode) -> Result<Lines> {
        // Determine terminal width, defaulting to 120 columns if unavailable
        let cols = terminal_size()
            .map(|(Width(w), _)| w as usize)
            .unwrap_or(120);
        let sep = "─".repeat(cols);
        let mut lines: Vec<Line> = Vec::new();

        // Top separator line
        lines.push(Line::sanitized(&sep));

        // Compute average duration or fallback to "n/a"
        let avg_str = if self.handled() > 0 {
            format!("{:.2}s", self.total_duration() / self.handled() as f64)
        } else {
            "n/a".into()
        };

        // Build stats line showing processed, pending, and average values
        let mut stats = format!(
            " Processed: {:<6} │ Pending: {:<6} │ Avg: {}",
            self.handled(),
            self.pending(),
            avg_str
        );
        // Pad right to fill the terminal width
        stats.push_str(&" ".repeat(cols.saturating_sub(UnicodeWidthStr::width(stats.as_str()))));
        lines.push(Line::sanitized(&stats));

        // Middle separator line
        lines.push(Line::sanitized(&sep));

        // Snapshot current tasks
        let running: Vec<_> = self.tasks.iter().map(|kv| kv.value().clone()).collect();
        let completed: Vec<_> = {
            let mut v = Vec::with_capacity(self.completed.len());
            while let Some(item) = self.completed.pop() {
                v.push(item);
            }
            for item in &v {
                let _ = self.completed.push(item.clone());
            }
            v
        };

        let max = *CURRENT_NUM_THREADS;
        let running_len = running.len();

        if running_len >= max {
            // If running tasks exceed or equal max threads, sort by start time and take latest `max` tasks
            let mut slice = running;
            slice.sort_by_key(|r| match r.state {
                TaskState::Indexing(t0) | TaskState::Transcoding(t0) => t0,
                TaskState::Done(_) | TaskState::Failed(_) => Instant::now(),
            });
            for t in slice.into_iter().rev().take(max).rev() {
                lines.push(Line::sanitized(&t.fmt()));
            }
        } else {
            // Otherwise, fill with completed tasks first, then running tasks
            let need = max - running_len;
            let start = completed.len().saturating_sub(need);
            for t in completed.iter().skip(start) {
                lines.push(Line::sanitized(&t.fmt()));
            }
            for t in running {
                lines.push(Line::sanitized(&t.fmt()));
            }
        }

        // Fill remaining lines with blank spaces to maintain dashboard height
        while lines.len() < max + 3 {
            let blank = " ".repeat(cols);
            lines.push(Line::sanitized(&blank));
        }
        lines.truncate(max + 3);

        Ok(Lines(lines))
    }
}
