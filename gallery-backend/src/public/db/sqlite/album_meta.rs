use crate::public::structure::{album::Album, database_struct::database::definition::Database};
use arrayvec::ArrayString;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};

use super::shares;

const GET_ALBUM_SQL: &str = include_str!("sql/get_album.sql");
const GET_ALL_ALBUMS_SQL: &str = include_str!("sql/get_all_albums.sql");
const GET_ALBUM_STATS_AGGREGATES_SQL: &str = include_str!("sql/get_album_stats_aggregates.sql");
const GET_ALBUM_COVER_SQL: &str = include_str!("sql/get_album_cover.sql");
const IS_OBJECT_IN_ALBUM_SQL: &str = include_str!("sql/is_object_in_album.sql");
const GET_ALBUM_TAGS_SQL: &str = include_str!("sql/get_album_tags.sql");
const CREATE_ALBUM_META_SQL: &str = include_str!("sql/create_album_meta.sql");

pub fn create_album_meta_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(CREATE_ALBUM_META_SQL)?;
    Ok(())
}

pub fn get_album(conn: &Connection, id: &str) -> rusqlite::Result<Option<Album>> {
    let mut stmt = conn.prepare(GET_ALBUM_SQL)?;

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
                share_list: shares::get_album_shares(conn, &id)?,
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
        let mut stmt_tags = conn.prepare(GET_ALBUM_TAGS_SQL)?;
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
    let mut stmt = conn.prepare(GET_ALL_ALBUMS_SQL)?;
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
    let mut stmt = conn.prepare(GET_ALBUM_STATS_AGGREGATES_SQL)?;

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
    let mut stmt = conn.prepare(GET_ALBUM_COVER_SQL)?;

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
    let mut stmt = conn.prepare(IS_OBJECT_IN_ALBUM_SQL)?;
    Ok(stmt.exists(params![album_id, object_id])?)
}
