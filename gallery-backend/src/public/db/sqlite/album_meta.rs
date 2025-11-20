use crate::public::structure::{album::Album, database_struct::database::definition::Database};
use arrayvec::ArrayString;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};

use super::album_shares;

pub fn create_album_meta_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS album_meta (
            album_id TEXT PRIMARY KEY,
            cover_id TEXT,
            user_defined_metadata TEXT NOT NULL DEFAULT '{}',
            item_count INTEGER NOT NULL DEFAULT 0,
            item_size INTEGER NOT NULL DEFAULT 0,
            start_time INTEGER,
            end_time INTEGER,
            FOREIGN KEY (album_id) REFERENCES nodes(id) ON DELETE CASCADE,
            FOREIGN KEY (cover_id) REFERENCES nodes(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS check_album_kind_insert
         BEFORE INSERT ON album_meta
         FOR EACH ROW
         BEGIN
             SELECT CASE
                 WHEN (SELECT kind FROM nodes WHERE id = NEW.album_id) != 'album'
                 THEN RAISE(ABORT, 'album_id must reference a node with kind = album')
             END;
         END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS check_album_kind_update
         BEFORE UPDATE ON album_meta
         FOR EACH ROW
         BEGIN
             SELECT CASE
                 WHEN (SELECT kind FROM nodes WHERE id = NEW.album_id) != 'album'
                 THEN RAISE(ABORT, 'album_id must reference a node with kind = album')
             END;
         END;",
        [],
    )?;

    Ok(())
}

pub fn get_album(conn: &Connection, id: &str) -> rusqlite::Result<Option<Album>> {
    let mut stmt = conn.prepare(
        "SELECT
        n.id,
        n.title,
        n.created_time,
        n.pending,
        n.width,
        n.height,
        am.start_time,
        am.end_time,
        n.last_modified_time,
        am.cover_id,
        n.thumbhash,
        am.user_defined_metadata,
        am.item_count,
        am.item_size
    FROM nodes n
    LEFT JOIN album_meta am ON n.id = am.album_id
    WHERE n.id = ? AND n.kind = 'album'",
    )?;

    let result = stmt
        .query_row(params![id], |row| {
            let id: String = row.get(0)?;
            let title: Option<String> = row.get(1)?;
            let created_time: i64 = row.get(2)?;
            let pending: bool = row.get(3)?;
            let width: u32 = row.get(4)?;
            let height: u32 = row.get(5)?;
            let start_time: Option<i64> = row.get(6)?;
            let end_time: Option<i64> = row.get(7)?;
            let last_modified_time: i64 = row.get(8)?;
            let cover_id: Option<String> = row.get(9)?;
            let thumbhash: Option<Vec<u8>> = row.get(10)?;
            let user_meta_json: String = row.get(11)?;
            let item_count: usize = row.get(12)?;
            let item_size: i64 = row.get(13)?;

            let user_defined_metadata: HashMap<String, Vec<String>> =
                serde_json::from_str(&user_meta_json).unwrap_or_default();

            Ok(Album {
                id: ArrayString::from(&id).unwrap_or_default(),
                title,
                created_time: created_time as u128,
                start_time: start_time.map(|t| t as u128),
                end_time: end_time.map(|t| t as u128),
                last_modified_time: last_modified_time as u128,
                cover: cover_id.map(|c| ArrayString::from(&c).unwrap_or_default()),
                thumbhash,
                user_defined_metadata,
                share_list: album_shares::get_album_shares(conn, &id)?,
                tag: HashSet::new(), // Will fill later
                width,
                height,
                item_count,
                item_size: item_size as u64,
                pending,
            })
        })
        .optional()?;

    if let Some(mut album) = result {
        // Calculate stats on-read
        let (count, size, _, _, cover_db) = get_album_stats(conn, id)?;

        album.item_count = count;
        album.item_size = size;

        // Validate cover
        let current_cover_valid = if let Some(cover_id) = &album.cover {
            is_object_in_album(conn, cover_id, id).unwrap_or(false)
        } else {
            false
        };

        if !current_cover_valid {
            if let Some(db) = cover_db {
                album.cover = Some(db.hash);
                album.thumbhash = Some(db.thumbhash);
                album.width = db.width;
                album.height = db.height;
            } else {
                album.cover = None;
                album.thumbhash = None;
                album.width = 0;
                album.height = 0;
            }
        }

        // Fetch tags
        let mut stmt_tags = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?")?;
        let tags_iter = stmt_tags.query_map(params![id], |row| row.get(0))?;
        for tag in tags_iter {
            album.tag.insert(tag?);
        }

        Ok(Some(album))
    } else {
        Ok(None)
    }
}

pub fn get_all_albums(conn: &Connection) -> rusqlite::Result<Vec<Album>> {
    let mut stmt = conn.prepare("SELECT id FROM nodes WHERE kind = 'album'")?;
    let ids_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut albums = Vec::new();
    for id in ids_iter {
        if let Ok(Some(album)) = get_album(conn, &id?) {
            albums.push(album);
        }
    }
    Ok(albums)
}

pub fn get_album_stats(
    conn: &Connection,
    album_id: &str,
) -> rusqlite::Result<(usize, u64, Option<u128>, Option<u128>, Option<Database>)> {
    // Aggregates
    let mut stmt = conn.prepare(
        "SELECT COUNT(*), IFNULL(SUM(nodes.size), 0), MIN(nodes.timestamp), MAX(nodes.timestamp)
         FROM album_items
         JOIN nodes ON album_items.item_id = nodes.id
         WHERE album_items.album_id = ?",
    )?;

    let (count, size, start, end) = stmt.query_row(params![album_id], |row| {
        Ok((
            row.get::<_, usize>(0)?,
            row.get::<_, i64>(1)? as u64,
            row.get::<_, Option<i64>>(2)?.map(|t| t as u128),
            row.get::<_, Option<i64>>(3)?.map(|t| t as u128),
        ))
    })?;

    if count == 0 {
        return Ok((0, 0, None, None, None));
    }

    // Cover (First item by timestamp)
    let mut stmt = conn.prepare(
        "SELECT nodes.id
         FROM album_items
         JOIN nodes ON album_items.item_id = nodes.id
         WHERE album_items.album_id = ?
         ORDER BY nodes.timestamp ASC
         LIMIT 1",
    )?;

    let cover_id: Option<String> = stmt
        .query_row(params![album_id], |row| row.get(0))
        .optional()?;

    let cover_db = if let Some(id) = cover_id {
        super::nodes::get_database(conn, &id)?
    } else {
        None
    };

    Ok((count, size, start, end, cover_db))
}

pub fn is_object_in_album(
    conn: &Connection,
    object_id: &str,
    album_id: &str,
) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM album_items WHERE album_id = ? AND item_id = ?")?;
    Ok(stmt.exists(params![album_id, object_id])?)
}
