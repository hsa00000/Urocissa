use crate::public::db::tree::TREE;
use crate::public::structure::album::{Album, Share};
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use anyhow::Result;
use arrayvec::ArrayString;
use rand::Rng;
use rand::distr::Alphanumeric;
use rocket::post;
use rocket::serde::json::Json;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json;
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
        let conn = TREE.get_connection().unwrap();
        match create_and_insert_share(&conn, create_share) {
            Ok(link) => Ok(link),
            Err(err) => Err(err),
        }
    })
    .await
    .unwrap()
}

fn create_and_insert_share(conn: &Connection, create_share: CreateShare) -> AppResult<String> {
    let album_opt: Option<Album> = conn
        .query_row(
            "SELECT * FROM album WHERE id = ?",
            [&*create_share.album_id],
            |row| Album::from_row(row),
        )
        .ok();

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
                exp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            album.share_list.insert(share_id, share);
            let share_list_json = serde_json::to_string(&album.share_list).unwrap();
            conn.execute(
                "UPDATE album SET share_list = ? WHERE id = ?",
                [&share_list_json, &*create_share.album_id],
            )
            .unwrap();
            Ok(link)
        }
        None => Err(anyhow::anyhow!("Album not found").into()),
    }
}
