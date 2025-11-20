#[derive(Debug)]
#[allow(dead_code)]
pub struct SchemaNode {
    pub id: String,
    pub kind: String, // 'image', 'video', 'album'
    pub created_time: i64,
    pub pending: bool,
    pub width: i32,
    pub height: i32,
    pub size: i64,
    pub timestamp: Option<i64>, // 可選，因為 SQL 中沒有 NOT NULL
}