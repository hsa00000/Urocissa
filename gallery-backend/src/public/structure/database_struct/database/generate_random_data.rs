use crate::{
    operations::hash::generate_random_hash,
    public::structure::database_struct::file_modify::FileModify,
};

use rand::Rng;

use std::collections::{BTreeMap, HashSet};

use super::definition::Database;

impl Database {
    pub fn generate_random_data() -> Self {
        let hash = generate_random_hash();

        let width = rand::rng().random_range(300..=600);
        let height = rand::rng().random_range(300..=600);

        Self {
            size: 0,
            hash,
            width,
            height,
            thumbhash: Vec::<u8>::new(),
            phash: Vec::<u8>::new(),
            ext_type: "image".to_string(),
            ext: "jpg".to_string(),
            exif_vec: BTreeMap::<String, String>::new(),
            tag: HashSet::<String>::new(),
            album: HashSet::new(),
            alias: vec![FileModify {
                file: String::from("/"),
                modified: 0,
                scan_time: 0,
            }],
            pending: false,
        }
    }
}
