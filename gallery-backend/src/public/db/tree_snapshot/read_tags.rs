use super::TreeSnapshot;
use crate::{public::db::tree::TREE, public::db::tree::read_tags::TagInfo};
use anyhow::Result;
impl TreeSnapshot {
    pub fn read_tags(&self) -> Result<Vec<TagInfo>> {
        let conn = TREE.get_connection()?;
        let mut stmt =
            conn.prepare("SELECT tag, COUNT(*) as count FROM tag_databases GROUP BY tag")?;
        let rows = stmt.query_map([], |row| {
            let tag: String = row.get(0)?;
            let count: usize = row.get(1)?;
            Ok(TagInfo { tag, number: count })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)
    }
}
