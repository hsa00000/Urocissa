use crate::public::db::sqlite::SQLITE;
use crate::router::{AppResult, GuardResult};
use crate::{
    public::structure::database_struct::database::definition::Database,
    router::fairing::guard_auth::GuardAuth,
};
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
    let objects = SQLITE.get_all_objects().unwrap_or_default();
    let byte_stream = ByteStream! {
        // Start the JSON array
        yield b"[".to_vec();
        let mut first = true;

        for database in objects {
            // Insert a comma if not the first element
            if !first {
                yield b",".to_vec();
            }
            first = false;

            // Build the ExportEntry
            let export = ExportEntry {
                key: database.hash.to_string(),
                value: database,
            };

            // Convert it to JSON
            let json_obj = match serde_json::to_string(&export) {
                Ok(s) => s,
                Err(_) => {
                    // Skip or handle the error
                    continue;
                }
            };

            // Stream it out
            yield json_obj.into_bytes();
        }

        // End the JSON array
        yield b"]".to_vec();
    };
    Ok(byte_stream)
}
