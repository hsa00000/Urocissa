#[derive(Debug)]
#[allow(dead_code)]
pub struct SchemaExif {
    pub node_id: String,
    pub tag: String,
    pub value: Option<String>,
}