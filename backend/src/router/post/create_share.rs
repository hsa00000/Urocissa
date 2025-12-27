use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Share;
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::{public::constant::redb::DATA_TABLE, router::GuardResult};
use anyhow::Result;
use arrayvec::ArrayString;
use rand::Rng;
use rand::distr::Alphanumeric;
use redb::{ReadableTable, WriteTransaction};
use rocket::post;
use rocket::serde::json::Json;
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
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    create_share: Json<CreateShare>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
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

fn create_and_insert_share(txn: &WriteTransaction, create_share: CreateShare) -> AppResult<String> {
    let mut data_table = txn.open_table(DATA_TABLE).unwrap();

    let album_opt = data_table
        .get(&*create_share.album_id)
        .unwrap()
        .and_then(|guard| {
            let abstract_data = guard.value();
            match abstract_data {
                AbstractData::Album(album) => Some(album),
                _ => None,
            }
        });

    match album_opt {
        Some(mut album) => {
            let link: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                .take(64)
                .map(char::from)
                .collect();
            let share_id = ArrayString::<64>::from(&link).unwrap();
            let share = Share {
                url: share_id,
                description: create_share.description,
                password: create_share.password,
                show_metadata: create_share.show_metadata,
                show_download: create_share.show_download,
                show_upload: create_share.show_upload,
                exp: create_share.exp,
            };
            album.metadata.share_list.insert(share_id, share);
            data_table.insert(&*create_share.album_id, AbstractData::Album(album)).unwrap();
            Ok(link)
        }
        None => Err(anyhow::anyhow!("Album not found").into()),
    }
}
