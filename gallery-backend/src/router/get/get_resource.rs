use crate::operations::transitor::abstract_data_to_database_timestamp_return;
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::TargetSet;
use crate::public::db::tree_snapshot::PendingTreeSnapshot;
use crate::public::db::write_behind::WRITE_BEHIND;
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::response::database_timestamp::DataBaseTimestampReturn;
use crate::router::claims::claims_timestamp::ClaimsTimestamp;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::get::get_prefetch::{Prefetch, insert_data_into_tree_snapshot};
use crate::router::{AppResult, GuardResult};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteResourceSnapshot {
    pub prefetch: Prefetch,
    pub token: String,
    pub data: DataBaseTimestampReturn,
}

fn validate_resource_id(resource_id: &str) -> AppResult<()> {
    if resource_id.len() != 64
        || !resource_id
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase())
    {
        return Err(AppError::new(ErrorKind::InvalidInput, "Invalid item ID"));
    }
    Ok(())
}

fn build_single_resource_snapshot(
    slot_ref: crate::public::db::tree::state::SlotRef,
    structural_epoch: u64,
    universe: usize,
) -> PendingTreeSnapshot {
    PendingTreeSnapshot {
        structural_epoch,
        universe,
        ordinals: vec![slot_ref.index()],
        targets: TargetSet::from_unique_slot_refs([slot_ref], universe),
        scrollbar: Vec::new(),
    }
}

fn create_resource_snapshot(resource_id: &str) -> AppResult<RouteResourceSnapshot> {
    validate_resource_id(resource_id)?;

    let (slot_ref, structural_epoch, universe) = {
        let state = TREE.state.read().map_err(|error| {
            AppError::new(
                ErrorKind::Internal,
                format!("Failed to read tree in memory: {error:?}"),
            )
        })?;
        let slot_ref = state.find(resource_id).ok_or_else(|| {
            AppError::new(
                ErrorKind::NotFound,
                format!("Item not found for id '{resource_id}'"),
            )
        })?;
        (slot_ref, state.structural_epoch(), state.arena.capacity())
    };

    let durable = TREE
        .store
        .read(|table| {
            table
                .get(resource_id)
                .map(|value| value.map(|value| value.into_value()))
        })
        .map_err(|error| AppError::from_err(ErrorKind::Database, error))?;
    let abstract_data = WRITE_BEHIND
        .logical_record(resource_id, durable)
        .ok_or_else(|| {
            AppError::new(
                ErrorKind::NotFound,
                format!("Item not found for id '{resource_id}'"),
            )
        })?;

    let snapshot = build_single_resource_snapshot(slot_ref, structural_epoch, universe);
    let (timestamp, data_length) = insert_data_into_tree_snapshot(snapshot);
    debug_assert_eq!(data_length, 1);

    let claims = ClaimsTimestamp::new(None, timestamp);
    let token = claims.encode();
    let data = abstract_data_to_database_timestamp_return(abstract_data, timestamp, true, true);

    Ok(RouteResourceSnapshot {
        prefetch: Prefetch {
            timestamp,
            locate_to: Some(0),
            data_length,
        },
        token,
        data,
    })
}

#[get("/get/resource/<resource_id>")]
pub async fn get_resource(
    auth: GuardResult<GuardAuth>,
    resource_id: String,
) -> AppResult<Json<RouteResourceSnapshot>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || create_resource_snapshot(&resource_id).map(Json))
        .await
        .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))?
}

#[cfg(test)]
mod tests {
    use super::{build_single_resource_snapshot, validate_resource_id};
    use crate::public::db::tree::state::SlotRef;
    use crate::public::error::ErrorKind;

    const VALID_MEDIA_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const VALID_ALBUM_ID: &str = "sntwjj2vnqq66wrxc5w5h1rn7t14k29d5wbjuj6abxhiun88881mtem1c06ez74u";

    #[test]
    fn accepts_media_and_album_ids() {
        assert!(validate_resource_id(VALID_MEDIA_ID).is_ok());
        assert!(validate_resource_id(VALID_ALBUM_ID).is_ok());
    }

    #[test]
    fn rejects_non_canonical_hashes() {
        for value in [
            "short",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde-",
            "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
        ] {
            let error = validate_resource_id(value).unwrap_err();
            assert_eq!(error.kind, ErrorKind::InvalidInput);
            assert_eq!(error.message, "Invalid item ID");
        }
    }

    #[test]
    fn direct_snapshot_maps_its_only_item_to_route_index_zero() {
        let slot_ref = SlotRef::new(17, 3);
        let snapshot = build_single_resource_snapshot(slot_ref, 41, 64);

        assert_eq!(snapshot.structural_epoch, 41);
        assert_eq!(snapshot.ordinals, vec![17]);
        assert_eq!(snapshot.targets.len(), 1);
        assert_eq!(snapshot.targets.slot_ref_for_ordinal(17), Some(slot_ref));
        assert!(snapshot.scrollbar.is_empty());
    }
}
