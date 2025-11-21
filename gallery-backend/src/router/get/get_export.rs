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
    // Collect all data synchronously
    let conn = crate::public::db::sqlite::DB_POOL.get().unwrap();
    let mut stmt = conn.prepare("SELECT * FROM database").unwrap();
    let rows = stmt.query_map([], |row| Database::from_row(row)).unwrap();
    let mut entries = Vec::new();
    for db_res in rows {
        if let Ok(db) = db_res {
            entries.push(ExportEntry {
                key: db.hash.to_string(),
                value: db,
            });
        }
    }

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
