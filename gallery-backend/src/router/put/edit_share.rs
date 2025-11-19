use crate::public::db::sqlite::SQLITE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Share;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::router::AppResult;
use anyhow::Result;
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
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<EditShare>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || -> Result<()> {
        if let Some(mut album) = SQLITE.get_album(json_data.album_id.as_str())? {
            album
                .share_list
                .insert(json_data.share.url, json_data.share.clone());
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![
                AbstractData::Album(album),
            ]));
        }
        Ok(())
    })
    .await
    .unwrap()?;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
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
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<DeleteShare>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || -> Result<()> {
        if let Some(mut album) = SQLITE.get_album(json_data.album_id.as_str())? {
            album.share_list.remove(&json_data.share_id);
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![
                AbstractData::Album(album),
            ]));
        }
        Ok(())
    })
    .await
    .unwrap()?;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
    Ok(())
}

