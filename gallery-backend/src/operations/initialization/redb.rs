use crate::public::constant::storage::get_data_path;
use std::fs;

pub fn initialize_file() {
    let root = get_data_path();

    for (file_name, label) in [
        ("temp_db.redb", "legacy tree cache"),
        ("temp_db_v4.redb", "tree cache"),
        ("cache_db.redb", "legacy query cache"),
        ("cache_db_v4.redb", "query cache"),
        ("expire_db.redb", "expire table"),
    ] {
        let db_path = root.join("db").join(file_name);
        if fs::metadata(&db_path).is_ok() {
            match fs::remove_file(&db_path) {
                Ok(()) => {
                    info!("Clear {label}");
                }
                Err(_) => {
                    error!("Fail to delete {label} {db_path:?}");
                }
            }
        }
    }
}
