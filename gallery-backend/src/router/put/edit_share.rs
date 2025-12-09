use crate::public::db::tree::TREE;
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::table::relations::album_share::Share;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
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
    tokio::task::spawn_blocking(move || {
        let conn = TREE.get_connection().unwrap();
        conn.execute(
            "UPDATE album_share SET 
                description = ?, 
                password = ?, 
                show_metadata = ?, 
                show_download = ?, 
                show_upload = ?, 
                exp = ?
            WHERE album_id = ? AND url = ?",
            (
                &json_data.share.description,
                &json_data.share.password,
                json_data.share.show_metadata,
                json_data.share.show_download,
                json_data.share.show_upload,
                json_data.share.exp,
                json_data.album_id.as_str(),
                json_data.share.url.as_str(),
            ),
        )
        .unwrap();
    })
    .await
    .unwrap();
    // UpdateTreeTask might not be needed if shares are not in the tree anymore,
    // but keeping it doesn't hurt if other things changed
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
    tokio::task::spawn_blocking(move || {
        let conn = TREE.get_connection().unwrap();
        conn.execute(
            "DELETE FROM album_share WHERE album_id = ? AND url = ?",
            [&*json_data.album_id, &*json_data.share_id],
        )
        .unwrap();
    })
    .await
    .unwrap();
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
    Ok(())
}
