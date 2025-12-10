use anyhow::{Result, anyhow};
use arrayvec::ArrayString;
use rand::{Rng, distr::Alphanumeric};
use redb::ReadableTable;
use rocket::{post, put};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::{AppResult, GuardResult};
use crate::api::fairings::guards::auth::GuardAuth;
use crate::api::fairings::guards::readonly::GuardReadOnlyMode;
use crate::background::actors::BATCH_COORDINATOR;
use crate::background::batchers::update_tree::UpdateTreeTask;
use crate::database::ops::tree::TREE;
use crate::database::schema::meta_album::META_ALBUM_TABLE;
use crate::database::schema::relations::album_share::{ALBUM_SHARE_TABLE, Share};

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateShare {
    pub album_id: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: u64,
}

#[post("/post/create_share", data = "<create_share>")]
pub async fn create_share(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    create_share: Json<CreateShare>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || {
        let create_share = create_share.into_inner();
        let mut txn = TREE.begin_write().unwrap();
        match create_and_insert_share(&mut txn, create_share) {
            Ok(link) => {
                txn.commit().unwrap();
                Ok(link)
            },
            Err(err) => Err(err),
        }
    })
    .await
    .unwrap()
}

fn create_and_insert_share(txn: &mut redb::WriteTransaction, create_share: CreateShare) -> AppResult<String> {
    // Check if album exists
    let album_table = txn.open_table(META_ALBUM_TABLE)?;
    let album_exists = album_table.get(&*create_share.album_id)?.is_some();

    if !album_exists {
        return Err(anyhow::anyhow!("Album not found").into());
    }

    let link: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .take(64)
        .map(char::from)
        .collect();
    
    let share_id = ArrayString::<64>::from(&link).unwrap();
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut share_table = txn.open_table(ALBUM_SHARE_TABLE)?;
    let share = Share {
        url: share_id,
        description: create_share.description,
        password: create_share.password,
        show_metadata: create_share.show_metadata,
        show_download: create_share.show_download,
        show_upload: create_share.show_upload,
        exp,
    };
    let encoded = bitcode::encode(&share);
    share_table.insert((create_share.album_id.as_str(), share_id.as_str()), encoded.as_slice())?;

    Ok(link)
}

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
        {
            let mut share_table = txn.open_table(ALBUM_SHARE_TABLE).unwrap();
            let share = Share {
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
        }
        txn.commit().unwrap();
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
            let mut share_table = txn.open_table(ALBUM_SHARE_TABLE).unwrap();
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

pub fn generate_share_routes() -> Vec<rocket::Route> {
    routes![
        create_share,
        edit_share,
        delete_share
    ]
}
