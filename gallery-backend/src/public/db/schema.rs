use rusqlite::Connection;
use crate::public::structure::database_struct::database::definition::Database;
use crate::public::structure::album::Album;
use crate::public::structure::relations::album_databases::AlbumDatabases;
use crate::public::structure::relations::tag_databases::TagDatabases;

pub fn create_all_tables(conn: &Connection) -> rusqlite::Result<()> {
    // 先建主表
    Database::create_database_table(conn)?;
    Album::create_album_table(conn)?;

    // 再建關聯表
    AlbumDatabases::create_table(conn)?;
    TagDatabases::create_table(conn)?;

    Ok(())
}