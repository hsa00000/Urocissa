use crate::looper::tree::TREE;
use crate::public::error_data::{ErrorData, handle_error};
use crate::structure::database_struct::database::definition::Database;
use crate::synchronizer::delete::delete_paths;
use arrayvec::ArrayString;
use blake3::Hasher;
use dashmap::DashMap;
use rayon::prelude::*;
use std::mem;
use std::panic::Location;
use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

pub fn validator<I>(vec_of_database: I) -> DashMap<ArrayString<64>, Database>
where
    I: ParallelIterator<Item = Database>,
{
    let duplicated_files_number = AtomicUsize::new(0);
    let dashmap_of_database: DashMap<ArrayString<64>, Database> = DashMap::new();
    let scaned_number = AtomicUsize::new(0);
    vec_of_database.for_each(|mut database| {
        match blake3_hasher(&database.source_path()) {
            Ok(hash) => {
                let read_table = TREE.api_read_tree();

                // File already in persistent database
                if let Some(guard) = read_table.get(&*hash).unwrap() {
                    let mut database_exist = guard.value();
                    let file_modify = mem::take(&mut database.alias[0]);
                    let path_to_delete = file_modify.file.clone().into();

                    database_exist.alias.push(file_modify);
                    TREE.insert_tree_api(&vec![database_exist]).unwrap();
                    TREE.tree_update();
                    scaned_number.fetch_add(1, Ordering::SeqCst);

                    delete_paths(vec![path_to_delete]);
                }
                // Duplicate file in current scan
                else if let Some(mut duplicated_database) = dashmap_of_database.get_mut(&hash) {
                    let file_modify = mem::take(&mut database.alias[0]);
                    let path_to_delete = file_modify.file.clone().into();

                    duplicated_database.alias.push(file_modify);
                    duplicated_files_number.fetch_add(1, Ordering::SeqCst);

                    delete_paths(vec![path_to_delete]);
                }
                // New file
                else {
                    database.hash = hash;
                    dashmap_of_database.insert(hash, database);
                }
            }
            Err(err) => handle_error(err),
        }
    });
    dashmap_of_database
}

fn blake3_hasher(file_path: &Path) -> Result<ArrayString<64>, ErrorData> {
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(err) => {
            return Err(ErrorData::new(
                err.to_string(),
                format!("Failed to read file"),
                None,
                Some(file_path.to_path_buf()),
                Location::caller(),
                None,
            ));
        }
    };
    let mut hasher = Hasher::new();
    let mut buffer = [0; 512 * 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Err(err) => {
                return Err(ErrorData::new(
                    err.to_string(),
                    format!("fail to read file"),
                    None,
                    Some(file_path.to_path_buf()),
                    Location::caller(),
                    None,
                ));
            }
        }
    }
    let hash = hasher.finalize();
    Ok(hash.to_hex())
}
