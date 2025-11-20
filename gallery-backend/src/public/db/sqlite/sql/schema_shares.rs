#[derive(Debug)]
#[allow(dead_code)]
pub struct SchemaShares {
    pub url: String,
    pub album_id: String,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: i64,
}