#[derive(Debug)]
#[allow(dead_code)]
pub struct SchemaImageMeta {
    pub node_id: String,
    pub thumbhash: Option<Vec<u8>>,
    pub phash: Option<Vec<u8>>,
}