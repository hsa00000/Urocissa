use crate::public::structure::tag_info::TagInfo;
use rusqlite::Connection;

const CREATE_NODE_TAGS_SQL: &str = include_str!("sql/create_nodes_tags.sql");

pub fn create_node_tags_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_NODE_TAGS_SQL, []).map(|_| ())
}

pub fn get_all_tags(conn: &Connection) -> rusqlite::Result<Vec<TagInfo>> {
    let mut stmt = conn.prepare(
        "SELECT tag, COUNT(*)
         FROM nodes_tags
         GROUP BY tag",
    )?;

    let iter = stmt.query_map([], |row| {
        Ok(TagInfo {
            tag: row.get(0)?,
            number: row.get(1)?,
        })
    })?;

    let mut tags = Vec::new();
    for tag in iter {
        tags.push(tag?);
    }
    Ok(tags)
}
