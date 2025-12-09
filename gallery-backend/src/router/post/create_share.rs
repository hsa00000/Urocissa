use crate::public::db::tree::TREE;
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use anyhow::Result;
use arrayvec::ArrayString;
use rand::Rng;
use rand::distr::Alphanumeric;
use redb::ReadableTable;
use rocket::post;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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
            Ok(link) => Ok(link),
            Err(err) => Err(err),
        }
    })
    .await
    .unwrap()
}

fn create_and_insert_share(txn: &mut redb::WriteTransaction, create_share: CreateShare) -> AppResult<String> {
    // Check if album exists
    let album_table = txn.open_table(crate::table::meta_album::META_ALBUM_TABLE)?;
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

    let mut share_table = txn.open_table(crate::table::relations::album_share::ALBUM_SHARE_TABLE)?;
    let share = crate::table::relations::album_share::Share {
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
