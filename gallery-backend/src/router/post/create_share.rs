use std::time::{SystemTime, UNIX_EPOCH};

use arrayvec::ArrayString;
use rand::Rng;
use rand::distr::Alphanumeric;
use redb::{ReadableTable, WriteTransaction};
use rocket::serde::json::Json;
use rocket::{http::Status, post};

use crate::public::album::Share;
use crate::public::redb::ALBUM_TABLE;
use crate::public::tree::TREE;
use crate::router::fairing::guard_auth::AuthGuard;
use crate::router::fairing::guard_read_only_mod::ReadOnlyModeGuard;
use serde::{Deserialize, Serialize};

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
    _auth: AuthGuard,
    _read_only_mode: ReadOnlyModeGuard,
    create_share: Json<CreateShare>,
) -> Result<String, Status> {
    tokio::task::spawn_blocking(move || {
        let create_share = create_share.into_inner();
        let txn = TREE.in_disk.begin_write().unwrap();
        match create_and_insert_share(&txn, create_share) {
            Ok(link) => {
                txn.commit().unwrap();
                return Ok(link);
            }
            Err(err) => return Err(err),
        }
    })
    .await
    .unwrap()
}

fn create_and_insert_share(
    txn: &WriteTransaction,
    create_share: CreateShare,
) -> Result<String, Status> {
    let mut album_table = txn.open_table(ALBUM_TABLE).unwrap();

    // TODO: simplfy this
    let album_opt = match album_table.get(&*create_share.album_id).unwrap() {
        Some(guard) => {
            let album = guard.value();
            Some(album)
        }
        _ => None,
    };

    match album_opt {
        Some(mut album) => {
            let link: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                .take(64)
                .map(char::from)
                .collect();
            let share = Share {
                url: ArrayString::<64>::from(&link).unwrap(),
                description: create_share.description,
                password: create_share.password,
                show_metadata: create_share.show_metadata,
                show_download: create_share.show_download,
                show_upload: create_share.show_upload,
                exp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            album.share_list.push(share);
            album_table.insert(&*create_share.album_id, album).unwrap();
            return Ok(link);
        }
        _ => {
            return Err(Status::NotFound);
        }
    }
}
