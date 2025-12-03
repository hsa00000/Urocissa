use crate::public::db::tree::TREE;
use crate::router::{AppResult, GuardResult};
use crate::public::structure::abstract_data::Database;
use crate::router::fairing::guard_auth::GuardAuth;
use rocket::get;
use rocket::response::stream::ByteStream;
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct ExportEntry {
    key: String,
    value: Database,
}

#[get("/get/get-export")]
pub async fn get_export(auth: GuardResult<GuardAuth>) -> AppResult<ByteStream![Vec<u8>]> {
    let _ = auth?;
    // Collect all data synchronously
    let entries = TREE.load_all_databases_from_db()?;
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
