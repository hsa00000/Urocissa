use crate::public::db::tree::TREE;
use crate::table::relations::album_share::Share;
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
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
        let txn = TREE.begin_write().unwrap();
        let mut share_table = txn.open_table(crate::table::relations::album_share::ALBUM_SHARE_TABLE).unwrap();
        let share = crate::table::relations::album_share::Share {
            url: json_data.share.url.clone(),
            description: json_data.share.description.clone(),
            password: json_data.share.password.clone(),
            show_metadata: json_data.share.show_metadata,
            show_download: json_data.share.show_download,
            show_upload: json_data.share.show_upload,
            exp: json_data.share.exp,
        };
        let encoded = bitcode::encode(&share);
        share_table.insert((json_data.album_id.as_str(), json_data.share.url.as_str()), encoded.as_slice()).unwrap();
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
        let txn = TREE.begin_write().unwrap();
        {
            let mut share_table = txn.open_table(crate::table::relations::album_share::ALBUM_SHARE_TABLE).unwrap();
            share_table.remove((json_data.album_id.as_str(), json_data.share_id.as_str())).unwrap();
        }
        txn.commit().unwrap();
    })
    .await
    .unwrap();
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
    Ok(())
}
