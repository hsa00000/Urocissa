use rocket::serde::json::Json;

use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::{AppResult, GuardResult};
use crate::tasks::actor::reindex::{ReindexJobStatus, reindex_job_statuses};

#[get("/get/reindex/jobs")]
pub fn get_reindex_jobs(auth: GuardResult<GuardAuth>) -> AppResult<Json<Vec<ReindexJobStatus>>> {
    let _ = auth?;
    Ok(Json(reindex_job_statuses()))
}
