pub mod fs_utils;
pub mod io_ext;

use std::path::{Path, PathBuf};

use crate::database::schema::object::ObjectType;

pub trait PathExt {
    fn ext_lower(&self) -> String;
}

impl PathExt for Path {
    fn ext_lower(&self) -> String {
        self.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default()
    }
}

pub fn compressed_path<H: AsRef<str>>(hash: H, obj_type: ObjectType) -> PathBuf {
    let hash_str = hash.as_ref();
    match obj_type {
        ObjectType::Image => PathBuf::from(format!(
            "./object/compressed/{}/{}.jpg",
            &hash_str[0..2],
            hash_str
        )),
        ObjectType::Video => PathBuf::from(format!(
            "./object/compressed/{}/{}.mp4",
            &hash_str[0..2],
            hash_str
        )),
        ObjectType::Album => panic!("Album has no compressed path"),
    }
}

pub fn thumbnail_path<H: AsRef<str>>(hash: H) -> PathBuf {
    let hash_str = hash.as_ref();
    PathBuf::from(format!(
        "./object/compressed/{}/{}.jpg",
        &hash_str[0..2],
        hash_str
    ))
}

pub fn imported_path<H: AsRef<str>>(hash: H, ext: impl AsRef<str>) -> PathBuf {
    let ext = ext.as_ref();
    let hash_str = hash.as_ref();
    PathBuf::from(format!(
        "./object/imported/{}/{}.{}",
        &hash_str[0..2],
        hash_str,
        ext
    ))
}
