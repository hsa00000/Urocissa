use crate::public::db::tree::TREE;
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Share;
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_tree::UpdateTreeTask;

use arrayvec::ArrayString;
use rocket::serde::{Deserialize, json::Json};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditShare {
    album_id: ArrayString<64>,
    share: Share,
}

#[put("/put/edit_share", format = "json", data = "<json_data>")]
pub async fn edit_share(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<EditShare>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        TREE.store.write(|data_table| {
            let album_opt = data_table
                .get(json_data.album_id.as_str())?
                .map(|guard| guard.value());

            if let Some(AbstractData::Album(mut album)) = album_opt {
                album
                    .metadata
                    .share_list
                    .insert(json_data.share.url, json_data.share.clone());
                data_table.insert_at(json_data.album_id.as_str(), &AbstractData::Album(album))?;
            }
            Ok::<(), AppError>(())
        })?;
        Ok(())
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update tree"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteShare {
    album_id: ArrayString<64>,
    share_id: ArrayString<64>,
}

#[put("/put/delete_share", format = "json", data = "<json_data>")]
pub async fn delete_share(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    json_data: Json<DeleteShare>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        TREE.store.write(|data_table| {
            let album_opt = data_table
                .get(json_data.album_id.as_str())?
                .map(|guard| guard.value());

            if let Some(AbstractData::Album(mut album)) = album_opt {
                album.metadata.share_list.remove(&json_data.share_id);
                data_table.insert_at(json_data.album_id.as_str(), &AbstractData::Album(album))?;
            }
            Ok::<(), AppError>(())
        })?;
        Ok(())
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to update tree"))?;
    Ok(())
}
