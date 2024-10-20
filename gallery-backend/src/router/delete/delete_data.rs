use crate::public::redb::DATA_TABLE;
use crate::public::tree::start_loop::SHOULD_RESET;
use crate::public::tree::TREE;
use crate::public::tree_snapshot::TREE_SNAPSHOT;
use redb::ReadableTable;
use rocket::http::Status;
use rocket::serde::{json::Json, Deserialize};
use std::sync::atomic::Ordering;
#[derive(Debug, Deserialize)]
pub struct DeleteList {
    #[serde(rename = "deleteList")]
    delete_list: Vec<usize>, // Keep this field private as it does not need to be accessed outside this struct
    timestamp: String,
}
#[delete("/delete/delete-data", format = "json", data = "<json_data>")]
pub async fn delete_data(json_data: Json<DeleteList>) -> Result<(), Status> {
    println!("get data {:?}", json_data);

    tokio::task::spawn_blocking(move || {
        let timestamp = &json_data.timestamp;
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(timestamp).unwrap();
        let txn = TREE.in_disk.begin_write().unwrap();
        {
            let mut table = txn.open_table(DATA_TABLE).unwrap();
            for index in &json_data.delete_list {
                let hash = tree_snapshot.get_hash(*index);
                let data = table.get(hash.as_str()).unwrap().unwrap().value();
                let compressed_path = data.compressed_path();
                let imported_path = data.imported_path();

                if std::fs::remove_file(&compressed_path).is_err() {
                    eprintln!("Failed to delete file at {:?}", compressed_path);
                    return Err(Status::InternalServerError);
                }
                if std::fs::remove_file(&imported_path).is_err() {
                    eprintln!("Failed to delete file at {:?}", imported_path);
                    return Err(Status::InternalServerError);
                }
                match table.remove(hash.as_str()) {
                    Ok(_) => (),
                    Err(err) => {
                        println!("{:?}", err);
                        return Err(Status::InternalServerError);
                    }
                };
            }
        }
        txn.commit().unwrap();
        SHOULD_RESET.store(true, Ordering::SeqCst);
        Ok(())
    })
    .await
    .unwrap()
}
