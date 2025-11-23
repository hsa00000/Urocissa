use super::definition::Database;
use std::path::PathBuf;

impl Database {
    pub fn source_path_string(&self) -> &str {
        &self.alias[0].file
    }
    pub fn source_path(&self) -> PathBuf {
        PathBuf::from(self.source_path_string())
    }
    pub fn imported_path_string(&self) -> String {
        format!(
            "./object/imported/{}/{}.{}",
            &self.hash[0..2],
            self.hash,
            self.ext
        )
    }
    pub fn compressed_path_string(&self) -> String {
        if self.ext_type == "image" {
            format!("./object/compressed/{}/{}.jpg", &self.hash[0..2], self.hash)
        } else {
            format!("./object/compressed/{}/{}.mp4", &self.hash[0..2], self.hash)
        }
    }
    pub fn imported_path(&self) -> PathBuf {
        PathBuf::from(self.imported_path_string())
    }

    pub fn compressed_path(&self) -> PathBuf {
        PathBuf::from(self.compressed_path_string())
    }
    pub fn thumbnail_path(&self) -> String {
        format!("./object/compressed/{}/{}.jpg", &self.hash[0..2], self.hash)
    }
    pub fn compressed_path_parent(&self) -> PathBuf {
        self.compressed_path()
            .parent()
            .expect("Path::new(&output_file_path_string).parent() fail")
            .to_path_buf()
    }
}
