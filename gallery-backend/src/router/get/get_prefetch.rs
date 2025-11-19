use crate::public::db::sqlite::SQLITE;
use crate::public::structure::album::ResolvedShare;
use crate::public::structure::expression::Expression;
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::claims::claims_timestamp::ClaimsTimestamp;
use crate::router::fairing::guard_share::GuardShare;

use anyhow::{Result, anyhow};
use bitcode::{Decode, Encode};
use log::info;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use std::time::{Instant, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Decode, Encode)]
#[serde(rename_all = "camelCase")]
pub struct Prefetch {
    pub timestamp: u128,
    pub locate_to: Option<usize>,
    pub data_length: usize,
}

impl Prefetch {
    fn new(timestamp: u128, locate_to: Option<usize>, data_length: usize) -> Self {
        Self {
            timestamp,
            locate_to,
            data_length,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Decode, Encode)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchReturn {
    pub prefetch: Prefetch,
    pub token: String,
    pub resolved_share_opt: Option<ResolvedShare>,
}

impl PrefetchReturn {
    fn new(prefetch: Prefetch, token: String, resolved_share_opt: Option<ResolvedShare>) -> Self {
        Self {
            prefetch,
            token,
            resolved_share_opt,
        }
    }
}

// -----------------------------------------------------------------------------
// ── Helper functions for each step ──────────────────────────────────────────
// -----------------------------------------------------------------------------

fn create_json_response(
    timestamp_millis: u128,
    locate_to_index: Option<usize>,
    reduced_data_vector_length: usize,
    resolved_share_option: Option<ResolvedShare>,
) -> Json<PrefetchReturn> {
    let json_start_time = Instant::now();

    let prefetch = Prefetch::new(
        timestamp_millis,
        locate_to_index,
        reduced_data_vector_length,
    );

    // Build response
    let claims = ClaimsTimestamp::new(resolved_share_option, timestamp_millis);
    let json = Json(PrefetchReturn::new(
        prefetch,
        claims.encode(),
        claims.resolved_share_opt,
    ));

    let duration = format!("{:?}", json_start_time.elapsed());
    info!(duration = &*duration; "Create JSON response");

    json
}

// -----------------------------------------------------------------------------
// ── Single prefetch function ─────────────────────────────────────────────────
// -----------------------------------------------------------------------------

fn execute_prefetch_logic(
    expression_option: Option<Expression>,
    locate_option: Option<String>,
    resolved_share_option: Option<ResolvedShare>,
) -> Result<Json<PrefetchReturn>> {
    // Start timer
    let start_time = Instant::now();

    let timestamp_millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

    let (hide_metadata, shared_album_id) = if let Some(resolved_share) = &resolved_share_option {
        (!resolved_share.share.show_metadata, Some(resolved_share.album_id.as_str()))
    } else {
        (false, None)
    };

    // Generate snapshot directly in SQLite
    let reduced_data_vector_length = SQLITE.generate_snapshot(
        timestamp_millis,
        &expression_option,
        hide_metadata,
        shared_album_id
    ).map_err(|e| anyhow!("SQLite snapshot generation failed: {}", e))?;

    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "Generate snapshot (SQLITE)");

    // Compute layout (locate index)
    let locate_to_index = if let Some(hash) = locate_option {
        SQLITE.get_snapshot_index(timestamp_millis, &hash)
            .map_err(|e| anyhow!("SQLite get_snapshot_index failed: {}", e))?
    } else {
        None
    };

    // Create and return JSON response
    let json = create_json_response(
        timestamp_millis,
        locate_to_index,
        reduced_data_vector_length,
        resolved_share_option,
    );

    // Total elapsed time
    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "(total time) Get_data_length complete");

    Ok(json)
}

#[post("/get/prefetch?<locate>", format = "json", data = "<query_data>")]
pub async fn prefetch(
    auth_guard: GuardResult<GuardShare>,
    query_data: Option<Json<Expression>>,
    locate: Option<String>,
) -> AppResult<Json<PrefetchReturn>> {
    let auth_guard = auth_guard?;
    // Combine album filter (if any) with the client‑supplied query.
    let mut combined_expression_option = query_data.map(|wrapper| wrapper.into_inner());
    let resolved_share_option = auth_guard.claims.get_share();

    if let Some(resolved_share) = &resolved_share_option {
        let album_filter_expression = Expression::Album(resolved_share.album_id);

        combined_expression_option = Some(match combined_expression_option {
            Some(client_expression) => {
                Expression::And(vec![album_filter_expression, client_expression])
            }
            None => album_filter_expression,
        });
    }

    // Execute on blocking thread
    let job_handle = tokio::task::spawn_blocking(move || {
        execute_prefetch_logic(combined_expression_option, locate, resolved_share_option)
    })
    .await??;

    Ok(job_handle)
}
