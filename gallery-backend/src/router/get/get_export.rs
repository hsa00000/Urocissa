use crate::operations::open_db::open_data_table;
use crate::router::{AppResult, GuardResult};
use crate::{
    public::structure::database_struct::database::definition::Database,
    router::fairing::guard_auth::GuardAuth,
};
use redb::ReadableTable;
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
    let data_table = open_data_table()?;
    let byte_stream = ByteStream! {
        // Open DB and prepare to iterate
        let iter = match data_table.iter() {
            Ok(it) => it,
            Err(_) => {
                yield b"{\"error\":\"failed to iterate\"}".to_vec();
                return;
            }
        };

        // Start the JSON array
        yield b"[".to_vec();
        let mut first = true;

        for entry_res in iter {
            let (key, value) = match entry_res {
                Ok(kv) => kv,
                Err(_) => {
                    // Skip or handle the error
                    continue;
                }
            };

            // Insert a comma if not the first element
            if !first {
                yield b",".to_vec();
            }
            first = false;

            // Build the ExportEntry
            let export = ExportEntry {
                key: key.value().to_string(),
                value: value.value().clone(),
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
