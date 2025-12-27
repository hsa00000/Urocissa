use std::fs;

pub fn initialize_file() {
    {
        let db_path = "./db/temp_db.redb";
        if fs::metadata(db_path).is_ok() {
            match fs::remove_file(db_path) {
                Ok(_) => {
                    info!("Clear tree cache");
                }
                Err(_) => {
                    error!("Fail to delete cache data ./db/temp_db.redb")
                }
            }
        }
    }
    {
        let db_path = "./db/cache_db.redb";
        if fs::metadata(db_path).is_ok() {
            match fs::remove_file(db_path) {
                Ok(_) => {
                    info!("Clear query cache");
                }
                Err(_) => {
                    error!("Fail to delete cache data ./db/cache_db.redb")
                }
            }
        }
    }
    {
        let db_path = "./db/expire_db.redb";
        if fs::metadata(db_path).is_ok() {
            match fs::remove_file(db_path) {
                Ok(_) => {
                    info!("Clear expire table");
                }
                Err(_) => {
                    error!("Fail to delete expire table ./db/expire_db.redb")
                }
            }
        }
    }
}
