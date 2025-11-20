use crate::public::structure::database_struct::{
    database::definition::Database, file_modify::FileModify,
};
use arrayvec::ArrayString;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{BTreeMap, HashSet};

const GET_DATABASE_SQL: &str = include_str!("sql/get_database.sql");
const GET_ALL_OBJECTS_SQL: &str = include_str!("sql/get_all_objects.sql");
const GET_NODE_TAGS_SQL: &str = include_str!("sql/get_nodes_tags.sql");
const GET_NODE_ALBUMS_SQL: &str = include_str!("sql/get_node_albums.sql");
const CREATE_NODES_SQL: &str = include_str!("sql/create_nodes.sql");

pub fn create_nodes_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(CREATE_NODES_SQL, []).map(|_| ())
}

pub fn get_database(conn: &Connection, hash: &str) -> rusqlite::Result<Option<Database>> {
    let mut stmt = conn.prepare(GET_DATABASE_SQL)?;

    let result = stmt
        .query_row(params![hash], |row| {
            let id: String = row.get(0)?;
            let size: u64 = row.get(1)?;
            let width: u32 = row.get(2)?;
            let height: u32 = row.get(3)?;
            let ext: String = row.get(4)?;
            let pending: bool = row.get(5)?;
            let thumbhash: Vec<u8> = row.get(6)?;
            let phash: Vec<u8> = row.get(7)?;

            let exif_vec: BTreeMap<String, String> = BTreeMap::new(); // Will fill later
            let alias: Vec<FileModify> = Vec::new(); // Will fill later

            Ok(Database {
                hash: ArrayString::from(&id).unwrap_or_default(),
                size,
                width,
                height,
                thumbhash,
                phash,
                ext,
                exif_vec,
                tag: HashSet::new(),   // Will fill later
                album: HashSet::new(), // Will fill later
                alias,
                ext_type: String::new(), // Default since column removed
                pending,
            })
        })
        .optional()?;

    if let Some(mut database) = result {
        // Fetch tags
        let mut stmt_tags = conn.prepare(GET_NODE_TAGS_SQL)?;
        let tags_iter = stmt_tags.query_map(params![hash], |row| row.get(0))?;
        for tag in tags_iter {
            database.tag.insert(tag?);
        }

        // Fetch albums
        let mut stmt_albums = conn.prepare(GET_NODE_ALBUMS_SQL)?;
        let albums_iter = stmt_albums.query_map(params![hash], |row| row.get(0))?;
        for album_id in albums_iter {
            let aid: String = album_id?;
            database
                .album
                .insert(ArrayString::from(&aid).unwrap_or_default());
        }

        // Fetch exif
        let mut stmt_exif = conn.prepare("SELECT tag, value FROM exif WHERE node_id = ?")?;
        let exif_iter = stmt_exif.query_map(params![hash], |row| {
            let tag: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((tag, value))
        })?;
        for exif_pair in exif_iter {
            let (tag, value) = exif_pair?;
            database.exif_vec.insert(tag, value);
        }

        // Fetch aliases
        let mut stmt_aliases =
            conn.prepare("SELECT file, modified, scan_time FROM aliases WHERE node_id = ?")?;
        let aliases_iter = stmt_aliases.query_map(params![hash], |row| {
            let file_path: String = row.get(0)?;
            let modified_time: u128 = row.get::<_, i64>(1)? as u128;
            let scan_time: u128 = row.get::<_, i64>(2)? as u128;
            Ok(FileModify {
                file: file_path,
                modified: modified_time,
                scan_time,
            })
        })?;
        for alias in aliases_iter {
            database.alias.push(alias?);
        }

        Ok(Some(database))
    } else {
        Ok(None)
    }
}

pub fn get_all_objects(conn: &Connection) -> rusqlite::Result<Vec<Database>> {
    let mut stmt = conn.prepare(GET_ALL_OBJECTS_SQL)?;
    let iter = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let size: u64 = row.get(1)?;
        let width: u32 = row.get(2)?;
        let height: u32 = row.get(3)?;
        let ext: String = row.get(4)?;
        let pending: bool = row.get(5)?;
        let thumbhash: Vec<u8> = row.get(6)?;
        let phash: Vec<u8> = row.get(7)?;

        let mut database = Database {
            hash: ArrayString::from(&id).unwrap_or_default(),
            size,
            width,
            height,
            thumbhash,
            phash,
            ext,
            exif_vec: BTreeMap::new(), // Will fill later
            tag: HashSet::new(),       // Will fill later
            album: HashSet::new(),     // Will fill later
            alias: Vec::new(),         // Will fill later
            ext_type: String::new(),   // Default since column removed
            pending,
        };

        // Fetch tags
        let mut stmt_tags = conn.prepare(GET_NODE_TAGS_SQL)?;
        let tags_iter = stmt_tags.query_map(params![&id], |row| row.get(0))?;
        for tag in tags_iter {
            database.tag.insert(tag?);
        }

        // Fetch albums
        let mut stmt_albums = conn.prepare(GET_NODE_ALBUMS_SQL)?;
        let albums_iter = stmt_albums.query_map(params![&id], |row| row.get(0))?;
        for album_id in albums_iter {
            let aid: String = album_id?;
            database
                .album
                .insert(ArrayString::from(&aid).unwrap_or_default());
        }

        // Fetch exif
        let mut stmt_exif = conn.prepare("SELECT tag, value FROM exif WHERE node_id = ?")?;
        let exif_iter = stmt_exif.query_map(params![&id], |row| {
            let tag: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((tag, value))
        })?;
        for exif_pair in exif_iter {
            let (tag, value) = exif_pair?;
            database.exif_vec.insert(tag, value);
        }

        // Fetch aliases
        let mut stmt_aliases = conn
            .prepare("SELECT file_path, modified_time, scan_time FROM aliases WHERE node_id = ?")?;
        let aliases_iter = stmt_aliases.query_map(params![&id], |row| {
            let file_path: String = row.get(0)?;
            let modified_time: u128 = row.get::<_, i64>(1)? as u128;
            let scan_time: u128 = row.get::<_, i64>(2)? as u128;
            Ok(FileModify {
                file: file_path,
                modified: modified_time,
                scan_time,
            })
        })?;
        for alias in aliases_iter {
            database.alias.push(alias?);
        }

        Ok(database)
    })?;

    let mut objects = Vec::new();
    for obj in iter {
        objects.push(obj?);
    }
    Ok(objects)
}
