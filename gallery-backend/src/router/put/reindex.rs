use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use serde::Deserialize;

use crate::process::media_pipeline::{MediaTaskPlan, ReindexOperation};
use crate::public::db::tree::TREE;
use crate::public::error::{AppError, ErrorKind};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::selection::{
    SelectionDescriptor, resolve_selection, resolved_selection_is_current,
};
use crate::router::{AppResult, GuardResult};
use crate::tasks::actor::reindex::{
    ReindexJobAccepted, ReindexJobStatus, cancel_reindex_job, enqueue_reindex_job,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexRequest {
    #[serde(default)]
    index_array: Vec<u32>,
    #[serde(default)]
    selection: Option<SelectionDescriptor>,
    timestamp: i64,
    #[serde(default)]
    operations: Option<Vec<ReindexOperation>>,
}

#[post("/put/reindex", format = "json", data = "<json_data>")]
pub async fn reindex(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<ReindexRequest>,
) -> AppResult<Custom<Json<ReindexJobAccepted>>> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = json_data.into_inner();
    let plan = requested_plan(data.operations)?;
    let selection = data
        .selection
        .unwrap_or_else(|| SelectionDescriptor::explicit(data.index_array));
    let resolved =
        tokio::task::spawn_blocking(move || resolve_selection(data.timestamp, selection))
            .await
            .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;
    let accepted = {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        if !resolved_selection_is_current(
            &state,
            resolved.identity_epoch,
            resolved.selection_epoch,
            &resolved.targets,
        ) {
            return Err(AppError::new(ErrorKind::Conflict, "selection is stale"));
        }
        let targets = state.media_targets(&resolved.targets);
        enqueue_reindex_job(&targets, plan)
    };
    Ok(Custom(Status::Accepted, Json(accepted)))
}

fn requested_plan(operations: Option<Vec<ReindexOperation>>) -> AppResult<MediaTaskPlan> {
    let operations = operations
        .filter(|operations| !operations.is_empty())
        .ok_or_else(|| {
            AppError::new(
                ErrorKind::InvalidInput,
                "operations is required and must contain at least one operation",
            )
        })?;
    MediaTaskPlan::new(operations)
        .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))
}

#[cfg(test)]
mod tests {
    use rocket::http::Status;

    use super::requested_plan;
    use crate::process::media_pipeline::ReindexOperation;

    #[test]
    fn missing_and_empty_operations_are_bad_requests() {
        assert_eq!(
            requested_plan(None).unwrap_err().http_status(),
            Status::BadRequest
        );
        assert_eq!(
            requested_plan(Some(Vec::new())).unwrap_err().http_status(),
            Status::BadRequest
        );
        assert!(requested_plan(Some(vec![ReindexOperation::Exif])).is_ok());
    }
}

#[post("/put/reindex/<job_id>/cancel")]
pub fn cancel_reindex(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    job_id: &str,
) -> AppResult<Json<ReindexJobStatus>> {
    let _ = auth?;
    let _ = read_only_mode?;
    cancel_reindex_job(job_id)
        .map(Json)
        .ok_or_else(|| AppError::new(ErrorKind::NotFound, "reindex job not found"))
}
