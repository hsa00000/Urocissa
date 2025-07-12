// tui.rs — Correctly using the DashMap<_, Arc<RwLock<_>>> pattern

use arrayvec::ArrayString;
use bytesize::ByteSize;
use dashmap::DashMap;
use std::{
    collections::VecDeque,
    mem,
    path::PathBuf,
    sync::{Arc, LazyLock, OnceLock, RwLock},
    time::Instant,
};
use superconsole::{Component, Dimensions, DrawMode, Line, Lines, SuperConsole};
use terminal_size::{terminal_size, Width};
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    time::{interval, Duration},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::structure::database_struct::database::definition::Database;

// ... (tui_task and other top-level statics are unchanged) ...
pub static LOGGER_TX: OnceLock<UnboundedSender<String>> = OnceLock::new();
pub static TASK_STORE: LazyLock<TaskStore> = LazyLock::new(TaskStore::new);
pub static MAX_ROWS: LazyLock<usize> = LazyLock::new(|| rayon::current_num_threads());

pub async fn tui_task(
    mut sc: SuperConsole,
    dashboard: Arc<RwLock<Dashboard>>,
    mut rx: UnboundedReceiver<String>,
) -> anyhow::Result<()> {
    let mut tick = interval(Duration::from_millis(200));
    loop {
        select! {
            Some(line) = rx.recv() => sc.emit(Lines(vec![superconsole::content::Line::unstyled(&line)?])),
            _ = tick.tick() => {
                let guard = dashboard.read().unwrap();
                sc.render(&*guard)?;
            }
        }
    }
}


/// The central storage for all tasks.
///
/// This uses the `DashMap<Key, Arc<RwLock<Value>>>` pattern, which is crucial for this application:
/// - `DashMap`: Provides fast, thread-safe access to the *collection* of tasks.
/// - `Arc`: Allows multiple threads (e.g., a worker and the TUI) to share ownership of a *single* task.
/// - `RwLock`: Provides interior mutability, allowing the worker to safely write changes to the task
///   while the TUI safely reads them, without holding up the entire DashMap.
pub struct TaskStore {
    tasks: DashMap<ArrayString<64>, Arc<RwLock<TaskRow>>>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
        }
    }

    /// Adds a new task to the store.
    pub fn add_task(&self, task_row: TaskRow) {
        let hash = task_row.state.database.hash;
        let task_arc = Arc::new(RwLock::new(task_row));
        self.tasks.insert(hash, task_arc);
    }

    /// Retrieves a thread-safe, shareable reference to a task by its hash.
    /// The DashMap's internal lock is only held for the duration of this lookup.
    pub fn get_task(&self, hash: &ArrayString<64>) -> Option<Arc<RwLock<TaskRow>>> {
        self.tasks.get(hash).map(|entry| entry.value().clone())
    }
}


// ... (FileType, TaskState, TaskStateMachine, TaskRow structs are unchanged from the previous version) ...
#[derive(Clone, Copy, Debug)]
pub enum FileType { Image, Video }
impl TryFrom<&str> for FileType {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> anyhow::Result<Self> {
        match s {
            "image" => Ok(FileType::Image),
            "video" => Ok(FileType::Video),
            _ => Err(anyhow::anyhow!("unknown file-type: {s}")),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TaskState { Pending, Indexing(Instant), Transcoding(Instant), Done(f64) }

pub struct TaskStateMachine {
    pub database: Database,
    pub state: TaskState,
}
impl TaskStateMachine {
    fn advance_state(&mut self) -> anyhow::Result<TaskState> {
        let file_type = FileType::try_from(self.database.ext_type.as_str())?;
        let current = mem::replace(&mut self.state, TaskState::Done(0.0));
        self.state = match current {
            TaskState::Pending => TaskState::Indexing(Instant::now()),
            TaskState::Indexing(t0) => match file_type {
                FileType::Image => TaskState::Done(t0.elapsed().as_secs_f64()),
                FileType::Video => TaskState::Transcoding(Instant::now()),
            },
            TaskState::Transcoding(t0) => TaskState::Done(t0.elapsed().as_secs_f64()),
            TaskState::Done(secs) => TaskState::Done(secs),
        };
        Ok(self.state.clone())
    }
}

pub struct TaskRow {
    pub path: PathBuf,
    pub state: TaskStateMachine,
    pub progress: Option<f64>,
}
impl TaskRow {
    pub fn advance_state(&mut self) -> anyhow::Result<TaskState> {
        if matches!(self.state.state, TaskState::Indexing(_)) {
            if let Ok(FileType::Video) = FileType::try_from(self.state.database.ext_type.as_str()) {
                self.progress = None;
            }
        }
        self.state.advance_state()
    }
    
    pub fn update_progress(&mut self, percent: f64) {
        self.progress = Some(percent.clamp(0.0, 100.0));
    }

    // fmt() and tail_ellipsis() are unchanged
    pub fn fmt(&self) -> String {
        const COL_STATUS: usize = 6;
        const COL_HASH: usize = 5;
        const DEFAULT_COLS: usize = 120;
        let margin = std::env::var("UROCISSA_TERM_MARGIN").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
        let cols = terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(DEFAULT_COLS);
        let status = match (&self.state.state, self.progress) {
            (TaskState::Transcoding(_), Some(p)) => format!("{:>5.1}%", p.min(100.0)),
            (TaskState::Done(_), _) => "✓".into(),
            (TaskState::Pending, _) => "·".into(),
            _ => "•".into(),
        };
        let status_col = format!("{:<COL_STATUS$}", status);
        let full_hash = self.state.database.hash.as_str();
        let short_hash = &full_hash[..COL_HASH.min(full_hash.len())];
        let hash_col = format!("{:>COL_HASH$}", short_hash);
        let secs = match self.state.state {
            TaskState::Pending => 0.0,
            TaskState::Indexing(t0) | TaskState::Transcoding(t0) => t0.elapsed().as_secs_f64(),
            TaskState::Done(d) => d,
        };
        let suffix = format!(" │ {:>6.1}s", secs);
        let prefix_w = COL_STATUS + 3 + COL_HASH + 3;
        let path_budget = cols.saturating_sub(prefix_w + UnicodeWidthStr::width(suffix.as_str()) + margin).max(5);
        let raw_path = self.path.display().to_string();
        let short_path = Self::tail_ellipsis(&raw_path, path_budget);
        let pad = " ".repeat(path_budget.saturating_sub(UnicodeWidthStr::width(short_path.as_str())));
        format!("{status_col} │ {hash_col} │ {short_path}{pad}{suffix}")
    }
    fn tail_ellipsis(s: &str, max: usize) -> String {
        if UnicodeWidthStr::width(s) <= max { return s.to_owned(); }
        let tail_len = max.saturating_sub(1);
        let mut acc = 0;
        let mut rev = String::new();
        for c in s.chars().rev() {
            let w = c.width().unwrap_or(0);
            if acc + w > tail_len { break; }
            acc += w;
            rev.push(c);
        }
        format!("…{}", rev.chars().rev().collect::<String>())
    }
}


// Dashboard struct and impl are unchanged from the previous version
pub struct Dashboard {
    pub pending_hashes: Vec<ArrayString<64>>,
    pub running_hashes: Vec<ArrayString<64>>,
    pub completed_hashes: VecDeque<ArrayString<64>>,
    pub handled: u64,
    pub db_bytes: u64,
}

impl Default for Dashboard { fn default() -> Self { Self::new() } }

impl Dashboard {
    pub fn new() -> Self {
        Self {
            pending_hashes: vec![],
            running_hashes: vec![],
            completed_hashes: VecDeque::new(),
            handled: 0,
            db_bytes: 0,
        }
    }
    pub fn add_pending_task(&mut self, hash: ArrayString<64>) { self.pending_hashes.push(hash); }
    pub fn promote_pending_to_running(&mut self) -> Option<ArrayString<64>> {
        if let Some(hash) = self.pending_hashes.pop() {
            self.running_hashes.push(hash);
            Some(hash)
        } else {
            None
        }
    }
    pub fn mark_task_completed(&mut self, hash: &ArrayString<64>) {
        if let Some(pos) = self.running_hashes.iter().position(|h| h == hash) {
            let finished_hash = self.running_hashes.remove(pos);
            self.completed_hashes.push_back(finished_hash);
            while self.completed_hashes.len() > *MAX_ROWS {
                self.completed_hashes.pop_front();
            }
            self.handled += 1;
        }
    }
}

// The Component impl for Dashboard is also unchanged
impl Component for Dashboard {
    fn draw_unchecked(&self, _: Dimensions, _: DrawMode) -> anyhow::Result<Lines> {
        let cols = terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(120);
        let sep = "─".repeat(cols);
        let mut lines: Vec<Line> = Vec::new();

        lines.push(vec![sep.clone()].try_into()?);
        let human_bytes = ByteSize(self.db_bytes).to_string();
        let running_count = self.running_hashes.len();
        let pending_count = self.pending_hashes.len();
        let mut stats = format!(
            "• Pending: {:<4} │ Running: {:<3} │ Processed: {:<6} │ DB size: {:>8}",
            pending_count, running_count, self.handled, human_bytes
        );
        stats.push_str(&" ".repeat(cols.saturating_sub(UnicodeWidthStr::width(stats.as_str()))));
        lines.push(vec![stats].try_into()?);
        lines.push(vec![sep.clone()].try_into()?);

        let render_task = |hash: &ArrayString<64>| -> Option<String> {
            TASK_STORE.get_task(hash).map(|task_arc| {
                task_arc.read().unwrap().fmt()
            })
        };

        let max = *MAX_ROWS;
        if running_count >= max {
            for hash in self.running_hashes.iter().rev().take(max).rev() {
                if let Some(formatted) = render_task(hash) { lines.push(vec![formatted].try_into()?); }
            }
        } else {
            let need_completed = max - running_count;
            let start = self.completed_hashes.len().saturating_sub(need_completed);
            for hash in self.completed_hashes.iter().skip(start) {
                if let Some(formatted) = render_task(hash) { lines.push(vec![formatted].try_into()?); }
            }
            for hash in &self.running_hashes {
                if let Some(formatted) = render_task(hash) { lines.push(vec![formatted].try_into()?); }
            }
        }

        const HEADER_HEIGHT: usize = 3;
        let target_len = HEADER_HEIGHT + max;
        while lines.len() < target_len {
            lines.push(vec![" ".repeat(cols)].try_into()?);
        }
        lines.truncate(target_len);

        Ok(Lines(lines))
    }
}