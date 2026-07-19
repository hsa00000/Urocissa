use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use anyhow::{Context, anyhow};
use arrayvec::ArrayString;
use futures::{FutureExt, future::join_all};
use mini_executor::{BatchTask, Task};
use serde::Serialize;
use tokio_rayon::AsyncThreadPool;
use uuid::Uuid;

use crate::process::artifact_publisher::ArtifactPublisher;
use crate::process::media_lock::lock_media;
use crate::process::media_pipeline::{
    MediaStage, MediaTaskPlan, ReindexOperation, ThumbnailPublishMode, execute_media_pipeline,
};
use crate::process::media_publish::publish_reindex_result;
use crate::public::constant::runtime::{CURRENT_NUM_THREADS, WORKER_RAYON_POOL};
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::{SlotRef, TargetSet};
use crate::public::db::write_behind::WRITE_BEHIND;
use crate::tasks::{BATCH_COORDINATOR, INDEX_COORDINATOR};

const MAX_TERMINAL_JOBS: usize = 20;
const MAX_ERROR_SUMMARIES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReindexJobState {
    Queued,
    Running,
    Completed,
    CompletedWithErrors,
    Canceled,
    Failed,
}

impl ReindexJobState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::CompletedWithErrors | Self::Canceled | Self::Failed
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexJobError {
    pub object_id: String,
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexJobStatus {
    pub job_id: String,
    pub state: ReindexJobState,
    pub queue_position: Option<usize>,
    pub operations: Vec<ReindexOperation>,
    pub total: usize,
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub cancel_requested: bool,
    pub errors: Vec<ReindexJobError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexJobAccepted {
    pub job_id: String,
    pub target_count: usize,
}

#[derive(Debug)]
struct ReindexJobProgress {
    state: ReindexJobState,
    processed: usize,
    succeeded: usize,
    failed: usize,
    skipped: usize,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    errors: Vec<ReindexJobError>,
}

#[derive(Debug)]
struct ReindexJob {
    id: String,
    targets: Vec<SlotRef>,
    plan: MediaTaskPlan,
    created_at: i64,
    cancel_requested: AtomicBool,
    progress: Mutex<ReindexJobProgress>,
}

impl ReindexJob {
    fn new(targets: &TargetSet, plan: MediaTaskPlan) -> Arc<Self> {
        Arc::new(Self {
            id: Uuid::new_v4().to_string(),
            targets: targets.iter().collect(),
            plan,
            created_at: now_millis(),
            cancel_requested: AtomicBool::new(false),
            progress: Mutex::new(ReindexJobProgress {
                state: ReindexJobState::Queued,
                processed: 0,
                succeeded: 0,
                failed: 0,
                skipped: 0,
                started_at: None,
                finished_at: None,
                errors: Vec::new(),
            }),
        })
    }

    fn begin(&self) -> bool {
        let mut progress = self.progress.lock().unwrap();
        if progress.state != ReindexJobState::Queued
            || self.cancel_requested.load(Ordering::Acquire)
        {
            return false;
        }
        progress.state = ReindexJobState::Running;
        progress.started_at = Some(now_millis());
        true
    }

    fn record(&self, outcome: ReindexObjectOutcome) {
        let mut progress = self.progress.lock().unwrap();
        progress.processed = progress.processed.saturating_add(1);
        match outcome {
            ReindexObjectOutcome::Succeeded => {
                progress.succeeded = progress.succeeded.saturating_add(1);
            }
            ReindexObjectOutcome::Skipped => {
                progress.skipped = progress.skipped.saturating_add(1);
            }
            ReindexObjectOutcome::Failed(error) => {
                progress.failed = progress.failed.saturating_add(1);
                if progress.errors.len() < MAX_ERROR_SUMMARIES {
                    progress.errors.push(error);
                }
            }
        }
    }

    fn finish(&self) {
        {
            let mut progress = self.progress.lock().unwrap();
            if progress.state.is_terminal() {
                return;
            }
            progress.state = if self.cancel_requested.load(Ordering::Acquire) {
                ReindexJobState::Canceled
            } else if progress.failed > 0 {
                ReindexJobState::CompletedWithErrors
            } else {
                ReindexJobState::Completed
            };
            progress.finished_at = Some(now_millis());
        }
        prune_terminal_jobs();
    }

    fn fail(&self, message: impl Into<String>) {
        {
            let mut progress = self.progress.lock().unwrap();
            if progress.state.is_terminal() {
                return;
            }
            progress.state = ReindexJobState::Failed;
            progress.finished_at = Some(now_millis());
            if progress.errors.len() < MAX_ERROR_SUMMARIES {
                progress.errors.push(ReindexJobError {
                    object_id: "unknown".to_string(),
                    stage: "job".to_string(),
                    message: message.into(),
                });
            }
        }
        prune_terminal_jobs();
    }

    fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::Release);
        let became_terminal = {
            let mut progress = self.progress.lock().unwrap();
            if progress.state == ReindexJobState::Queued {
                progress.state = ReindexJobState::Canceled;
                progress.finished_at = Some(now_millis());
                true
            } else {
                false
            }
        };
        if became_terminal {
            prune_terminal_jobs();
        }
    }

    fn status(&self) -> ReindexJobStatus {
        let progress = self.progress.lock().unwrap();
        ReindexJobStatus {
            job_id: self.id.clone(),
            state: progress.state,
            queue_position: None,
            operations: self.plan.operations().to_vec(),
            total: self.targets.len(),
            processed: progress.processed,
            succeeded: progress.succeeded,
            failed: progress.failed,
            skipped: progress.skipped,
            created_at: self.created_at,
            started_at: progress.started_at,
            finished_at: progress.finished_at,
            cancel_requested: self.cancel_requested.load(Ordering::Acquire),
            errors: progress.errors.clone(),
        }
    }
}

static REINDEX_JOBS: LazyLock<Mutex<VecDeque<Arc<ReindexJob>>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

pub fn enqueue_reindex_job(targets: &TargetSet, plan: MediaTaskPlan) -> ReindexJobAccepted {
    let job = ReindexJob::new(targets, plan);
    let accepted = ReindexJobAccepted {
        job_id: job.id.clone(),
        target_count: job.targets.len(),
    };
    REINDEX_JOBS.lock().unwrap().push_back(Arc::clone(&job));
    BATCH_COORDINATOR.execute_batch_detached(ReindexJobTask { job });
    accepted
}

pub fn reindex_job_statuses() -> Vec<ReindexJobStatus> {
    let jobs = REINDEX_JOBS
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut active = Vec::new();
    let mut terminal = Vec::new();
    let mut next_queue_position = 1;
    for job in jobs {
        let mut status = job.status();
        if status.state == ReindexJobState::Queued {
            status.queue_position = Some(next_queue_position);
            next_queue_position += 1;
        }
        if status.state.is_terminal() {
            terminal.push(status);
        } else {
            active.push(status);
        }
    }
    terminal.sort_unstable_by_key(|status| std::cmp::Reverse(status.finished_at));
    active.extend(terminal.into_iter().take(MAX_TERMINAL_JOBS));
    active
}

pub fn cancel_reindex_job(job_id: &str) -> Option<ReindexJobStatus> {
    let job = REINDEX_JOBS
        .lock()
        .unwrap()
        .iter()
        .find(|job| job.id == job_id)
        .cloned()?;
    job.request_cancel();
    Some(job.status())
}

fn prune_terminal_jobs() {
    let mut jobs = REINDEX_JOBS.lock().unwrap();
    prune_terminal_queue(&mut jobs);
}

fn prune_terminal_queue(jobs: &mut VecDeque<Arc<ReindexJob>>) {
    while jobs
        .iter()
        .filter(|job| job.progress.lock().unwrap().state.is_terminal())
        .count()
        > MAX_TERMINAL_JOBS
    {
        let oldest = jobs
            .iter()
            .enumerate()
            .filter_map(|(index, job)| {
                let progress = job.progress.lock().unwrap();
                progress
                    .state
                    .is_terminal()
                    .then_some((index, progress.finished_at.unwrap_or(i64::MIN)))
            })
            .min_by_key(|(_, finished_at)| *finished_at)
            .map(|(index, _)| index);
        let Some(oldest) = oldest else {
            break;
        };
        jobs.remove(oldest);
    }
}

pub struct ReindexJobTask {
    job: Arc<ReindexJob>,
}

impl BatchTask for ReindexJobTask {
    async fn batch_run(jobs: Vec<Self>) {
        // mini_executor preserves channel receive order. Keeping this loop
        // sequential makes the submitted jobs a strict FIFO queue.
        for task in jobs {
            let job = task.job;
            if AssertUnwindSafe(run_job(Arc::clone(&job)))
                .catch_unwind()
                .await
                .is_err()
            {
                job.fail("reindex job worker panicked");
            }
        }
    }
}

async fn run_job(job: Arc<ReindexJob>) {
    if !job.begin() {
        return;
    }
    let wave_size = (*CURRENT_NUM_THREADS).max(1);
    for wave in job.targets.chunks(wave_size) {
        if job.cancel_requested.load(Ordering::Acquire) {
            break;
        }
        let handles = wave
            .iter()
            .copied()
            .map(|slot_ref| {
                INDEX_COORDINATOR.execute_detached(ReindexObjectTask {
                    job_id: job.id.clone(),
                    slot_ref,
                    plan: job.plan.clone(),
                })
            })
            .collect::<Vec<_>>();
        for outcome in join_all(handles).await {
            match outcome {
                Ok(outcome) => job.record(outcome),
                Err(error) => job.record(ReindexObjectOutcome::Failed(ReindexJobError {
                    object_id: "unknown".to_string(),
                    stage: MediaStage::Publish.as_str().to_string(),
                    message: format!("object task failed to join: {error}"),
                })),
            }
        }
    }
    job.finish();
}

struct ReindexObjectTask {
    job_id: String,
    slot_ref: SlotRef,
    plan: MediaTaskPlan,
}

impl Task for ReindexObjectTask {
    type Output = ReindexObjectOutcome;

    async fn run(self) -> Self::Output {
        let Some(object_id) = object_id_for_slot(self.slot_ref) else {
            return ReindexObjectOutcome::Skipped;
        };
        let _media_guard = lock_media(object_id).await;
        if object_id_for_slot(self.slot_ref) != Some(object_id) {
            return ReindexObjectOutcome::Skipped;
        }
        let job_id = self.job_id;
        let slot_ref = self.slot_ref;
        let plan = self.plan;
        WORKER_RAYON_POOL
            .spawn_async(move || process_object(&job_id, slot_ref, object_id, &plan))
            .await
    }
}

enum ReindexObjectOutcome {
    Succeeded,
    Skipped,
    Failed(ReindexJobError),
}

fn process_object(
    job_id: &str,
    slot_ref: SlotRef,
    object_id: ArrayString<64>,
    plan: &MediaTaskPlan,
) -> ReindexObjectOutcome {
    match process_object_inner(job_id, slot_ref, object_id, plan) {
        Ok(true) => ReindexObjectOutcome::Succeeded,
        Ok(false) => ReindexObjectOutcome::Skipped,
        Err((stage, error)) => ReindexObjectOutcome::Failed(ReindexJobError {
            object_id: object_id.to_string(),
            stage: stage.as_str().to_string(),
            message: format!("{error:#}"),
        }),
    }
}

fn process_object_inner(
    job_id: &str,
    slot_ref: SlotRef,
    object_id: ArrayString<64>,
    plan: &MediaTaskPlan,
) -> std::result::Result<bool, (MediaStage, anyhow::Error)> {
    let durable = TREE
        .store
        .read(|reader| {
            Ok::<_, anyhow::Error>(
                reader
                    .get(object_id.as_str())?
                    .map(crate::storage::store::RecordValue::into_value),
            )
        })
        .map_err(|error| (MediaStage::Publish, error))?;
    let data = WRITE_BEHIND
        .logical_record_for_slot(Some(slot_ref), object_id.as_str(), durable)
        .ok_or_else(|| {
            (
                MediaStage::Publish,
                anyhow!("object no longer exists at its captured generation"),
            )
        })?;
    if !plan.has_applicable_operation(&data) {
        return Ok(false);
    }

    let token = format!("reindex-{job_id}-{}", slot_ref.raw());
    let mut publisher = ArtifactPublisher::new(token);
    let result = execute_media_pipeline(
        &data,
        plan,
        &mut publisher,
        ThumbnailPublishMode::ReplaceExisting,
    )
    .map_err(|error| (error.stage, error.source))?;
    publish_reindex_result(slot_ref, object_id, plan, &result, publisher)
        .context("failed to publish reindex result")
        .map_err(|error| (MediaStage::Publish, error))?;
    Ok(true)
}

fn object_id_for_slot(slot_ref: SlotRef) -> Option<ArrayString<64>> {
    TREE.state
        .read()
        .ok()?
        .get(slot_ref)
        .map(|record| record.id)
}

fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::{
        MAX_ERROR_SUMMARIES, MAX_TERMINAL_JOBS, ReindexJob, ReindexJobError, ReindexJobState,
        ReindexJobStatus, ReindexObjectOutcome, prune_terminal_queue,
    };
    use crate::process::media_pipeline::{MediaTaskPlan, ReindexOperation};
    use crate::public::db::tree::state::TargetSet;

    #[test]
    fn job_status_uses_camel_case_wire_names() {
        let status = ReindexJobStatus {
            job_id: "job".to_string(),
            state: ReindexJobState::CompletedWithErrors,
            queue_position: None,
            operations: vec![ReindexOperation::FileSize],
            total: 1,
            processed: 1,
            succeeded: 0,
            failed: 1,
            skipped: 0,
            created_at: 1,
            started_at: Some(2),
            finished_at: Some(3),
            cancel_requested: false,
            errors: vec![ReindexJobError {
                object_id: "object".to_string(),
                stage: "publish".to_string(),
                message: "failed".to_string(),
            }],
        };
        let value = serde_json::to_value(status).unwrap();
        assert_eq!(value["state"], "completedWithErrors");
        assert_eq!(value["queuePosition"], serde_json::Value::Null);
        assert_eq!(value["cancelRequested"], false);
        assert_eq!(value["errors"][0]["objectId"], "object");
    }

    #[test]
    fn registry_retains_active_jobs_and_only_twenty_newest_terminal_jobs() {
        let mut jobs = VecDeque::new();
        for finished_at in 0..(MAX_TERMINAL_JOBS + 5) {
            let job = ReindexJob::new(
                &TargetSet::default(),
                MediaTaskPlan::new(vec![ReindexOperation::Exif]).unwrap(),
            );
            {
                let mut progress = job.progress.lock().unwrap();
                progress.state = ReindexJobState::Completed;
                progress.finished_at = Some(i64::try_from(finished_at).unwrap());
            }
            jobs.push_back(job);
        }
        let active = ReindexJob::new(
            &TargetSet::default(),
            MediaTaskPlan::new(vec![ReindexOperation::Exif]).unwrap(),
        );
        let active_id = active.id.clone();
        jobs.push_front(active);

        prune_terminal_queue(&mut jobs);

        assert_eq!(jobs.len(), MAX_TERMINAL_JOBS + 1);
        assert!(jobs.iter().any(|job| job.id == active_id));
        let oldest_retained = jobs
            .iter()
            .filter_map(|job| job.progress.lock().unwrap().finished_at)
            .min()
            .unwrap();
        assert_eq!(oldest_retained, 5);
    }

    #[test]
    fn queued_and_running_cancellation_have_distinct_boundaries() {
        let queued = ReindexJob::new(
            &TargetSet::default(),
            MediaTaskPlan::new(vec![ReindexOperation::Exif]).unwrap(),
        );
        queued.request_cancel();
        assert_eq!(queued.status().state, ReindexJobState::Canceled);
        assert_eq!(queued.status().processed, 0);

        let running = ReindexJob::new(
            &TargetSet::default(),
            MediaTaskPlan::new(vec![ReindexOperation::Exif]).unwrap(),
        );
        assert!(running.begin());
        running.record(ReindexObjectOutcome::Succeeded);
        running.request_cancel();
        assert_eq!(running.status().state, ReindexJobState::Running);
        running.finish();
        let status = running.status();
        assert_eq!(status.state, ReindexJobState::Canceled);
        assert_eq!(status.succeeded, 1);
    }

    #[test]
    fn object_failures_are_partial_and_error_summaries_are_bounded() {
        let job = ReindexJob::new(
            &TargetSet::default(),
            MediaTaskPlan::new(vec![ReindexOperation::Exif]).unwrap(),
        );
        assert!(job.begin());
        job.record(ReindexObjectOutcome::Succeeded);
        for index in 0..(MAX_ERROR_SUMMARIES + 5) {
            job.record(ReindexObjectOutcome::Failed(ReindexJobError {
                object_id: index.to_string(),
                stage: "metadata".to_string(),
                message: "failed".to_string(),
            }));
        }
        job.finish();
        let status = job.status();
        assert_eq!(status.state, ReindexJobState::CompletedWithErrors);
        assert_eq!(status.succeeded, 1);
        assert_eq!(status.failed, MAX_ERROR_SUMMARIES + 5);
        assert_eq!(status.errors.len(), MAX_ERROR_SUMMARIES);
    }

    #[test]
    fn fatal_job_failure_uses_the_failed_terminal_state() {
        let job = ReindexJob::new(
            &TargetSet::default(),
            MediaTaskPlan::new(vec![ReindexOperation::Exif]).unwrap(),
        );
        assert!(job.begin());
        job.fail("worker stopped");
        let status = job.status();
        assert_eq!(status.state, ReindexJobState::Failed);
        assert_eq!(status.errors[0].stage, "job");
        assert_eq!(status.errors[0].message, "worker stopped");
    }
}
