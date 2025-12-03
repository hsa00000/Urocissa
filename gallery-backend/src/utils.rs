use std::path::Path;

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
