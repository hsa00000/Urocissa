use crate::public::structure::{
    database_struct::{database::definition::Database, file_modify::FileModify},
};
use arrayvec::ArrayString;
use rusqlite::{Connection, params, OptionalExtension};
use std::collections::{BTreeMap, HashSet};

pub fn create_nodes_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL CHECK (kind IN ('image', 'video', 'album')),
            title TEXT,
            created_time INTEGER NOT NULL,
            last_modified_time INTEGER,
            pending BOOLEAN NOT NULL DEFAULT 0,
            width INTEGER NOT NULL DEFAULT 0,
            height INTEGER NOT NULL DEFAULT 0,
            size INTEGER,
            timestamp INTEGER,
            thumbhash BLOB,
            phash BLOB,
            exif TEXT,
            alias TEXT
        )",
        [],
    ).map(|_| ())
}

pub fn get_database(conn: &Connection, hash: &str) -> rusqlite::Result<Option<Database>> {
    let mut stmt = conn.prepare("SELECT n.id, n.size, n.width, n.height, e.ext, n.pending, n.thumbhash, n.phash, n.exif, n.alias FROM nodes n LEFT JOIN extensions e ON n.id = e.node_id WHERE n.id = ? AND n.kind IN ('image', 'video')")?;

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
            let exif_json: String = row.get(8)?;
            let alias_json: String = row.get(9)?;

            let exif_vec: BTreeMap<String, String> =
                serde_json::from_str(&exif_json).unwrap_or_default();
            let alias: Vec<FileModify> = serde_json::from_str(&alias_json).unwrap_or_default();

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
        let mut stmt_tags = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?")?;
        let tags_iter = stmt_tags.query_map(params![hash], |row| row.get(0))?;
        for tag in tags_iter {
            database.tag.insert(tag?);
        }

        // Fetch albums
        let mut stmt_albums =
            conn.prepare("SELECT album_id FROM album_items WHERE item_id = ?")?;
        let albums_iter = stmt_albums.query_map(params![hash], |row| row.get(0))?;
        for album_id in albums_iter {
            let aid: String = album_id?;
            database
                .album
                .insert(ArrayString::from(&aid).unwrap_or_default());
        }

        Ok(Some(database))
    } else {
        Ok(None)
    }
}

pub fn get_all_objects(conn: &Connection) -> rusqlite::Result<Vec<Database>> {
    let mut stmt = conn.prepare("SELECT n.id, n.size, n.width, n.height, e.ext, n.pending, n.thumbhash, n.phash, n.exif, n.alias FROM nodes n LEFT JOIN extensions e ON n.id = e.node_id WHERE n.kind IN ('image', 'video')")?;
    let iter = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let size: u64 = row.get(1)?;
        let width: u32 = row.get(2)?;
        let height: u32 = row.get(3)?;
        let ext: String = row.get(4)?;
        let pending: bool = row.get(5)?;
        let thumbhash: Vec<u8> = row.get(6)?;
        let phash: Vec<u8> = row.get(7)?;
        let exif_json: String = row.get(8)?;
        let alias_json: String = row.get(9)?;

        let exif_vec: BTreeMap<String, String> =
            serde_json::from_str(&exif_json).unwrap_or_default();
        let alias: Vec<FileModify> = serde_json::from_str(&alias_json).unwrap_or_default();

        let mut database = Database {
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
        };

        // Fetch tags
        let mut stmt_tags = conn.prepare("SELECT tag FROM node_tags WHERE node_id = ?")?;
        let tags_iter = stmt_tags.query_map(params![&id], |row| row.get(0))?;
        for tag in tags_iter {
            database.tag.insert(tag?);
        }

        // Fetch albums
        let mut stmt_albums =
            conn.prepare("SELECT album_id FROM album_items WHERE item_id = ?")?;
        let albums_iter = stmt_albums.query_map(params![&id], |row| row.get(0))?;
        for album_id in albums_iter {
            let aid: String = album_id?;
            database
                .album
                .insert(ArrayString::from(&aid).unwrap_or_default());
        }

        Ok(database)
    })?;

    let mut objects = Vec::new();
    for obj in iter {
        objects.push(obj?);
    }
    Ok(objects)
}