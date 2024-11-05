use arrayvec::ArrayString;
use rand::distributions::Alphanumeric;
use rand::Rng;
use redb::ReadableTable;
use rocket::serde::json::Json;
use rocket::{http::Status, post};

use serde::{Deserialize, Serialize};

use crate::public::album::Album;
use crate::public::redb::{ALBUM_TABLE, DATA_TABLE};
use crate::public::tree::TREE;

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
pub struct CreateAlbum {
    pub title: Option<String>,
    pub elements: Vec<ArrayString<64>>,
}

#[post("/post/create_album", data = "<create_album>")]
pub async fn create_album(create_album: Json<CreateAlbum>) -> Result<(), Status> {
    let create_album = create_album.into_inner();
    let album_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .take(64)
        .map(char::from)
        .collect();
    let album_id = ArrayString::<64>::from(&album_id).unwrap();

    let album_database = Album::new(album_id, create_album.title);
    let txn = TREE.in_disk.begin_write().unwrap();
    {
        let mut album_table = txn.open_table(ALBUM_TABLE).unwrap();

        album_table
            .insert(album_id.as_str(), &album_database)
            .unwrap();

        let mut data_table = txn.open_table(DATA_TABLE).unwrap();

        create_album.elements.iter().for_each(|hash| {
            let mut data = data_table.get(hash.as_str()).unwrap().unwrap().value();
            data.album.insert(album_id);

            data_table.insert(&*data.hash, &data).unwrap();
        });
    }
    txn.commit().unwrap();
    Ok(())
}