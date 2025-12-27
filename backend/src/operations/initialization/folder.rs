use std::path::PathBuf;

pub fn initialize_folder() {
    std::fs::create_dir_all(PathBuf::from("./db")).unwrap();
    std::fs::create_dir_all(PathBuf::from("./object/imported")).unwrap();
    std::fs::create_dir_all(PathBuf::from("./object/compressed")).unwrap();
    std::fs::create_dir_all(PathBuf::from("./upload")).unwrap();
}
