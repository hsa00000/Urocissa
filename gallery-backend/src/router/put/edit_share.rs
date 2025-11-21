use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::{Album, Share};
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use arrayvec::ArrayString;
use rocket::serde::{Deserialize, json::Json};
use rusqlite::Connection;
use serde_json;
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
        let conn = Connection::open("./gallery.db").unwrap();
        if let Ok(mut album) = conn.query_row(
            "SELECT * FROM album WHERE id = ?",
            [&*json_data.album_id],
            |row| Album::from_row(row),
        ) {
            album
                .share_list
                .insert(json_data.share.url, json_data.share.clone());
            let share_list_json = serde_json::to_string(&album.share_list).unwrap();
            conn.execute(
                "UPDATE album SET share_list = ? WHERE id = ?",
                [&share_list_json, &*json_data.album_id],
            )
            .unwrap();
        }
    })
    .await
    .unwrap();
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
        let abstract_data = TREE.load_from_db(&json_data.album_id).unwrap();
        if let AbstractData::Album(mut album) = abstract_data {
            album.share_list.remove(&json_data.share_id);
            let share_list_json = serde_json::to_string(&album.share_list).unwrap();
            let conn = TREE.get_connection().unwrap();
            conn.execute(
                "UPDATE album SET share_list = ? WHERE id = ?",
                [&share_list_json, &*json_data.album_id],
            )
            .unwrap();
        }
    })
    .await
    .unwrap();
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
    Ok(())
}
