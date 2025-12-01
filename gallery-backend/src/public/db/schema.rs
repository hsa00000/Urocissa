use crate::table::album::AlbumSchema;
use crate::table::database::DatabaseSchema;
use crate::table::relations::album_databases::AlbumDatabasesTable;
use crate::table::relations::album_share::AlbumShareTable;
use crate::table::relations::database_alias::DatabaseAliasTable;
use crate::table::relations::database_exif::DatabaseExifTable;
use crate::table::relations::tag_databases::TagDatabasesTable;
use rusqlite::Connection;

pub fn create_all_tables(conn: &Connection) -> rusqlite::Result<()> {
    // 先建主表
    DatabaseSchema::create_table(conn)?;
    AlbumSchema::create_table(conn)?;

    // 再建關聯表
    AlbumDatabasesTable::create_table(conn)?;
    AlbumShareTable::create_table(conn)?;
    DatabaseAliasTable::create_table(conn)?;
    TagDatabasesTable::create_table(conn)?;
    DatabaseExifTable::create_table(conn)?;

    Ok(())
}
