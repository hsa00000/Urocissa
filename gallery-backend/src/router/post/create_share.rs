use crate::public::db::tree::TREE;
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::Share;
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::{router::GuardResult, storage::store::RecordWriter};

use arrayvec::ArrayString;
use rand::RngExt;
use rand::distr::Alphanumeric;
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
    pub exp: i64,
}

#[post("/post/create_share", data = "<create_share>")]
pub async fn create_share(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    create_share: Json<CreateShare>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || {
        let create_share = create_share.into_inner();
        TREE.store
            .write(|data_table| create_and_insert_share(data_table, create_share))
    })
    .await
    .map_err(|e| AppError::from_err(ErrorKind::Internal, e.into()))?
}

fn create_and_insert_share(
    data_table: &mut RecordWriter<'_>,
    create_share: CreateShare,
) -> AppResult<String> {
    let album_opt = data_table
        .get(create_share.album_id.as_str())?
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
            let share_id = ArrayString::<64>::from(&link)
                .map_err(|_| AppError::new(ErrorKind::Internal, "Failed to create share ID"))?;
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
            data_table.insert_at(create_share.album_id.as_str(), &AbstractData::Album(album))?;
            Ok(link)
        }
        None => Err(AppError::new(ErrorKind::NotFound, "Album not found")),
    }
}
