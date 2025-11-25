use crate::public::structure::album::Album;
use crate::public::structure::database::definition::DatabaseSchema;
use crate::public::structure::relations::album_databases::AlbumDatabases;
use crate::public::structure::relations::database_alias::DatabaseAlias;
use crate::public::structure::relations::exif_databases::DatabaseExif;
use crate::public::structure::relations::tag_databases::TagDatabases;
use rusqlite::Connection;

pub fn create_all_tables(conn: &Connection) -> rusqlite::Result<()> {
    // 先建主表
    DatabaseSchema::create_database_table(conn)?;
    Album::create_album_table(conn)?;

    // 再建關聯表
    AlbumDatabases::create_table(conn)?;
    DatabaseAlias::create_table(conn)?;
    TagDatabases::create_table(conn)?;
    DatabaseExif::create_table(conn)?;

    Ok(())
}
