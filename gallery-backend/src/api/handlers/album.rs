use anyhow::Result;
use arrayvec::ArrayString;
use rand::{Rng, distr::Alphanumeric};
use rocket::{get, post, put};
use rocket::serde::json::Json;
use rocket::response::stream::ByteStream;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use redb::ReadableTable;

use crate::api::{AppResult, GuardResult};
use crate::api::fairings::guards::auth::GuardAuth;
use crate::api::fairings::guards::readonly::GuardReadOnlyMode;
use crate::api::fairings::guards::share::GuardShare;
use crate::background::actors::BATCH_COORDINATOR;
use crate::background::batchers::flush_tree::{FlushOperation, FlushTreeTask};
use crate::background::batchers::update_tree::UpdateTreeTask;
use crate::background::processors::transitor::index_to_hash;
use crate::database::ops::snapshot::tree::TREE_SNAPSHOT;
use crate::database::ops::tree::TREE;
use crate::database::schema::album::AlbumCombined;
use crate::database::schema::meta_album::AlbumMetadataSchema;
use crate::database::schema::object::{ObjectSchema, ObjectType};
use crate::database::schema::relations::album_data::AlbumItemSchema;
use crate::models::entity::abstract_data::AbstractData;

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlbum {
    pub title: Option<String>,
    pub elements_index: Vec<usize>,
    pub timestamp: u128,
}

#[post("/post/create_empty_album")]
pub async fn create_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    let album_id = create_album_internal(None).await?;

    Ok(album_id.to_string())
}

#[post("/post/create_non_empty_album", data = "<create_album>")]
pub async fn create_non_empty_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    create_album: Json<CreateAlbum>,
) -> AppResult<String> {
    let _ = auth?;
    let _ = read_only_mode?;
    let create_album = create_album.into_inner();
    let album_id = create_album_internal(create_album.title).await?;
    create_album_elements(
        album_id,
        create_album.elements_index,
        create_album.timestamp,
    )
    .await?;

    Ok(album_id.to_string())
}

async fn create_album_internal(title: Option<String>) -> Result<ArrayString<64>> {
    let start_time = Instant::now();

    let album_id = generate_random_hash();
    let object = ObjectSchema::new(album_id, ObjectType::Album);
    let metadata = AlbumMetadataSchema::new(album_id, title);
    let album = AbstractData::Album(AlbumCombined { object, metadata });
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(vec![album]))
        .await?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;

    info!(duration = &*format!("{:?}", start_time.elapsed()); "Create album");
    Ok(album_id)
}

async fn create_album_elements(
    album_id: ArrayString<64>,
    elements_index: Vec<usize>,
    timestamp: u128,
) -> Result<()> {
    let flush_ops = tokio::task::spawn_blocking(move || -> Result<Vec<FlushOperation>> {
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&timestamp).unwrap();
        let mut flush_ops = Vec::new();
        for idx in elements_index {
            let hash = index_to_hash(&tree_snapshot, idx)?;
            flush_ops.push(FlushOperation::InsertAlbum(AlbumItemSchema {
                album_id: album_id.to_string(),
                hash: hash.to_string(),
            }));
        }
        Ok(flush_ops)
    })
    .await??;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask {
            operations: flush_ops,
        })
        .await?;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;
    // Album stats are now updated by database triggers

    Ok(())
}

/// Generate a random 64-character lowercase alphanumeric hash
pub fn generate_random_hash() -> ArrayString<64> {
    let hash: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .take(64)
        .map(char::from)
        .collect();

    ArrayString::<64>::from(&hash).unwrap()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditAlbumsData {
    index_array: Vec<usize>,
    add_albums_array: Vec<ArrayString<64>>,
    remove_albums_array: Vec<ArrayString<64>>,
    timestamp: u128,
}

#[put("/put/edit_album", format = "json", data = "<json_data>")]
pub async fn edit_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<EditAlbumsData>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;

    // 在 blocking 執行緒產生所有要寫入的 payload 與受影響相簿
    let flush_ops = tokio::task::spawn_blocking(move || -> Result<Vec<FlushOperation>> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        let mut flush_ops = Vec::new();
        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            for album_id in &json_data.add_albums_array {
                flush_ops.push(FlushOperation::InsertAlbum(AlbumItemSchema {
                    album_id: album_id.to_string(),
                    hash: hash.to_string(),
                }));
            }
            for album_id in &json_data.remove_albums_array {
                flush_ops.push(FlushOperation::RemoveAlbum(AlbumItemSchema {
                    album_id: album_id.to_string(),
                    hash: hash.to_string(),
                }));
            }
        }

        Ok(flush_ops)
    })
    .await
    .map_err(|e| anyhow::anyhow!("join error: {e}"))??;

    // 單次等待版本
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask {
            operations: flush_ops,
        })
        .await?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;

    // 受影響相簿：全部等待
    // Album stats are now updated by database triggers
    // No need to manually update albums

    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumCover {
    pub album_id: ArrayString<64>,
    pub cover_hash: ArrayString<64>,
}

#[put("/put/set_album_cover", data = "<set_album_cover>")]
pub async fn set_album_cover(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    set_album_cover: Json<SetAlbumCover>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || {
        let set_album_cover_inner = set_album_cover.into_inner();
        let album_id = set_album_cover_inner.album_id;
        let cover_hash = set_album_cover_inner.cover_hash;

        let data_opt = TREE.load_data_from_hash(&cover_hash).unwrap();
        let _data = data_opt.unwrap();
        let cover_str = cover_hash.as_str();

        // 修正：分別更新 object 與 meta_album
        let tx = TREE.begin_write().unwrap();
        {
            // 1. 更新 meta_album 的 cover
            let album_data = {
                let temp_table = tx.open_table(crate::database::schema::meta_album::META_ALBUM_TABLE).unwrap();
                temp_table.get(&*album_id).unwrap().map(|bytes| bytes.value().to_vec())
            };
            if let Some(album_bytes) = album_data {
                let mut album: crate::database::schema::meta_album::AlbumMetadataSchema = bitcode::decode(&album_bytes).unwrap();
                album.cover = Some(ArrayString::from(cover_str).unwrap());
                let encoded = bitcode::encode(&album);
                let mut table = tx.open_table(crate::database::schema::meta_album::META_ALBUM_TABLE).unwrap();
                table.insert(&*album_id, encoded.as_slice()).unwrap();
            }
        }

        let album_data = TREE.load_data_from_hash(&album_id).unwrap().unwrap();

        // 2. 更新 object 的 thumbhash
        let thumbhash = match &album_data {
            AbstractData::Image(i) => i.object.thumbhash.clone(),
            AbstractData::Video(v) => v.object.thumbhash.clone(),
            AbstractData::Album(_) => None,
        };
        {
            let object_data = {
                let temp_table = tx.open_table(crate::database::schema::object::OBJECT_TABLE).unwrap();
                temp_table.get(&*album_id).unwrap().map(|bytes| bytes.value().to_vec())
            };
            if let Some(object_bytes) = object_data {
                let mut object: crate::database::schema::object::ObjectSchema = bitcode::decode(&object_bytes).unwrap();
                object.thumbhash = thumbhash;
                let encoded = bitcode::encode(&object);
                let mut object_table = tx.open_table(crate::database::schema::object::OBJECT_TABLE).unwrap();
                object_table.insert(&*album_id, encoded.as_slice()).unwrap();
            }
        }

        tx.commit().unwrap();
    })
    .await
    .unwrap();
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumTitle {
    pub album_id: ArrayString<64>,
    pub title: Option<String>,
}

#[put("/put/set_album_title", data = "<set_album_title>")]
pub async fn set_album_title(
    auth: GuardResult<GuardShare>,
    read_only_mode: Result<GuardReadOnlyMode>,
    set_album_title: Json<SetAlbumTitle>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || {
        let set_album_title_inner = set_album_title.into_inner();
        let album_id = set_album_title_inner.album_id;

        let txn = TREE.begin_write().unwrap();
        {
            let album_data = {
                let temp_table = txn.open_table(crate::database::schema::meta_album::META_ALBUM_TABLE).unwrap();
                temp_table.get(&*album_id).unwrap().map(|bytes| bytes.value().to_vec())
            };
            if let Some(album_bytes) = album_data {
                let mut album: crate::database::schema::meta_album::AlbumMetadataSchema = bitcode::decode(&album_bytes).unwrap();
                album.title = set_album_title_inner.title;
                let encoded = bitcode::encode(&album);
                let mut table = txn.open_table(crate::database::schema::meta_album::META_ALBUM_TABLE).unwrap();
                table.insert(&*album_id, encoded.as_slice()).unwrap();
            }
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

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetUserDefinedDescription {
    pub index: usize,
    pub description: Option<String>,
    pub timestamp: u128,
}

#[put(
    "/put/set_user_defined_description",
    data = "<set_user_defined_description>"
)]
pub async fn set_user_defined_description(
    auth: GuardResult<GuardShare>,
    read_only_mode: Result<GuardReadOnlyMode>,
    set_user_defined_description: Json<SetUserDefinedDescription>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || -> Result<()> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&set_user_defined_description.timestamp)
            .unwrap();
        let hash = index_to_hash(&tree_snapshot, set_user_defined_description.index)?;
        let abstract_data = TREE.load_from_db(&hash)?;

        let mut operations = Vec::new();
        let new_desc = set_user_defined_description.description.clone();

        match abstract_data {
            AbstractData::Image(mut i) => {
                i.object.description = new_desc;
                operations.push(FlushOperation::InsertAbstractData(AbstractData::Image(i)));
            }
            AbstractData::Video(mut v) => {
                v.object.description = new_desc;
                operations.push(FlushOperation::InsertAbstractData(AbstractData::Video(v)));
            }
            AbstractData::Album(mut alb) => {
                alb.object.description = new_desc;
                operations.push(FlushOperation::InsertAbstractData(AbstractData::Album(alb)));
            }
        }

        if !operations.is_empty() {
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask { operations });
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

#[derive(Debug, Serialize)]
pub struct ExportEntry {
    key: String,
    value: AbstractData,
}

#[get("/get/get-export")]
pub async fn get_export(auth: GuardResult<GuardAuth>) -> AppResult<ByteStream![Vec<u8>]> {
    let _ = auth?;
    // Collect all data synchronously
    let entries = TREE.load_all_data_from_db()?;
    let entries = entries
        .into_iter()
        .map(|db| ExportEntry {
            key: db.hash().to_string(),
            value: db,
        })
        .collect::<Vec<_>>();

    let byte_stream = ByteStream! {
        // Start the JSON array
        yield b"[".to_vec();
        let mut first = true;

        for export in entries {
            // Insert a comma if not the first element
            if !first {
                yield b",".to_vec();
            }
            first = false;

            // Convert it to JSON
            let json_obj = match serde_json::to_string(&export) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Stream it out
            yield json_obj.into_bytes();
        }

        // End the JSON array
        yield b"]".to_vec();
    };
    Ok(byte_stream)
}

pub fn generate_album_routes() -> Vec<rocket::Route> {
    routes![
        create_empty_album,
        create_non_empty_album,
        edit_album,
        set_album_cover,
        set_album_title,
        set_user_defined_description,
        get_export
    ]
}
