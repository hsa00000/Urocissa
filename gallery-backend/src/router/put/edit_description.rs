use std::sync::atomic::Ordering;

use rocket::serde::{Deserialize, json::Json};
use serde::Serialize;

use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::public::db::write_behind::{DirtyOperation, WRITE_BEHIND};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::object::next_mutation_timestamp;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::selection::{SelectionDescriptor, resolve_selection};
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetUserDefinedDescription {
    pub index: u32,
    #[serde(default)]
    pub selection: Option<SelectionDescriptor>,
    pub description: Option<String>,
    pub timestamp: i64,
}

#[put(
    "/put/set_user_defined_description",
    data = "<set_user_defined_description>"
)]
pub async fn set_user_defined_description(
    auth: GuardResult<GuardShare>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    set_user_defined_description: Json<SetUserDefinedDescription>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let data = set_user_defined_description.into_inner();
    let selection = data
        .selection
        .unwrap_or_else(|| SelectionDescriptor::explicit(vec![data.index]));
    let resolved =
        tokio::task::spawn_blocking(move || resolve_selection(data.timestamp, selection))
            .await
            .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))??;
    if resolved.len != 1 {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "description edit requires exactly one item",
        ));
    }
    let target = resolved.targets.iter().next().expect("one resolved target");
    let structural_epoch = resolved.structural_epoch;
    let description = data.description;
    let operation = DirtyOperation::Description {
        target,
        value: description.clone(),
    };
    let universe = crate::public::db::tree::TREE
        .state
        .read()
        .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?
        .arena
        .capacity();
    let touch_preview = DirtyOperation::Touch {
        targets: crate::public::db::tree::state::TargetSet::from_slot_refs([target], universe),
        changed_at: 0,
    };
    let bytes = operation.estimated_bytes() + touch_preview.estimated_bytes();
    WRITE_BEHIND.reserve(bytes).await?;
    let mut state = crate::public::db::tree::TREE.state.write().map_err(|_| {
        WRITE_BEHIND.release_reservation(bytes);
        AppError::new(ErrorKind::Internal, "tree state lock poisoned")
    })?;
    if state.structural_epoch() != structural_epoch {
        WRITE_BEHIND.release_reservation(bytes);
        return Err(AppError::new(
            ErrorKind::Conflict,
            "selection became stale before publication",
        ));
    }
    let changed_at = next_mutation_timestamp();
    let touch_targets =
        crate::public::db::tree::state::TargetSet::from_slot_refs([target], universe);
    state.edit_cached_album_objects(&touch_targets, changed_at, |object| {
        object.description.clone_from(&description);
    });
    let touch = DirtyOperation::Touch {
        targets: touch_targets,
        changed_at,
    };
    WRITE_BEHIND.enqueue_reserved(operation, bytes);
    WRITE_BEHIND.enqueue_reserved(touch, 0);
    VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    drop(state);
    Ok(())
}
