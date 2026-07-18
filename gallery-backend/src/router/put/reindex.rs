use rocket::http::Status;
use rocket::serde::json::Json;
use serde::Deserialize;

use crate::process::info::{regenerate_metadata_for_image, regenerate_metadata_for_video};
use crate::public::db::tree::{TREE, state::TargetSet};
use crate::public::db::write_behind::WRITE_BEHIND;
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::selection::{SelectionDescriptor, resolve_selection};
use crate::router::{AppResult, GuardResult};
use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::Ordering;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateData {
    #[serde(default)]
    index_array: Vec<usize>,
    #[serde(default)]
    selection: Option<SelectionDescriptor>,
    timestamp: i64,
}

#[post("/put/reindex", format = "json", data = "<json_data>")]
pub async fn reindex(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<RegenerateData>,
) -> AppResult<Status> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = json_data.into_inner();
    let selection = data
        .selection
        .unwrap_or_else(|| SelectionDescriptor::explicit(data.index_array));
    let resolved =
        tokio::task::spawn_blocking(move || resolve_selection(data.timestamp, &selection))
            .await
            .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;
    let structural_epoch = resolved.structural_epoch;
    let targets = resolved.targets;
    tokio::task::spawn_blocking(move || -> AppResult<()> {
        let _persistence_guard = TREE.persistence_lock.lock().unwrap();
        {
            let state = TREE
                .state
                .read()
                .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
            if state.structural_epoch() != structural_epoch {
                return Err(AppError::new(
                    ErrorKind::Conflict,
                    "selection became stale before reindex",
                ));
            }
        }

        let mut slots = targets.iter();
        loop {
            let chunk = slots.by_ref().take(4_096).collect::<Vec<_>>();
            if chunk.is_empty() {
                break;
            }
            // Keep the logical mutation lock from overlay read through delta
            // retirement. New edits cannot be accidentally absorbed by this
            // reindex after its durable record has been assembled.
            let mut state = TREE
                .state
                .write()
                .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
            let pairs = chunk
                .iter()
                .filter_map(|slot_ref| state.get(*slot_ref).map(|record| (*slot_ref, record.id)))
                .collect::<Vec<_>>();
            let updated = TREE.store.read(|reader| {
                let mut updated = Vec::with_capacity(pairs.len());
                for (slot_ref, id) in &pairs {
                    let durable = reader.get(id.as_str())?.map(|value| value.into_value());
                    let Some(mut data) =
                        WRITE_BEHIND.logical_record_for_slot(Some(*slot_ref), id.as_str(), durable)
                    else {
                        continue;
                    };
                    match &data {
                        AbstractData::Image(_) => regenerate_metadata_for_image(&mut data)
                            .map_err(|error| AppError::from_err(ErrorKind::IO, error))?,
                        AbstractData::Video(_) => regenerate_metadata_for_video(&mut data)
                            .map_err(|error| AppError::from_err(ErrorKind::IO, error))?,
                        AbstractData::Album(_) => continue,
                    }
                    updated.push((*slot_ref, data));
                }
                Ok::<_, AppError>(updated)
            })?;
            TREE.store
                .write(|writer| {
                    for (_, data) in &updated {
                        writer.insert(data)?;
                    }
                    Ok::<(), anyhow::Error>(())
                })
                .map_err(|error| AppError::from_err(ErrorKind::Database, error))?;
            let updated_targets = TargetSet::from_slot_refs(
                updated.iter().map(|(slot_ref, _)| *slot_ref),
                state.arena.capacity(),
            );
            WRITE_BEHIND.cancel_targets(&updated_targets, &BTreeSet::new());
            let updated_records = updated
                .into_iter()
                .map(|(_, data)| data)
                .collect::<Vec<_>>();
            state.apply_batch(&updated_records, &HashSet::new());
        }
        crate::public::db::tree::VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
    .await
    .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;
    Ok(Status::Ok)
}
