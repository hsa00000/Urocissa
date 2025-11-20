use crate::public::structure::tag_info::TagInfo;
use rusqlite::Connection;

pub fn create_node_tags_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS node_tags (
            node_id TEXT,
            tag TEXT,
            PRIMARY KEY (node_id, tag),
            FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
        )",
        [],
    ).map(|_| ())
}

pub fn get_all_tags(conn: &Connection) -> rusqlite::Result<Vec<TagInfo>> {
    let mut stmt = conn.prepare(
        "SELECT tag, COUNT(*)
         FROM node_tags
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