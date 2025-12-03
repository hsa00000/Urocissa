use crate::table::object::ObjectSchema;
use crate::table::meta_image::ImageMetadataSchema;
use crate::table::meta_video::VideoMetadataSchema;
use crate::table::meta_album::AlbumMetadataSchema;
use crate::table::relations::album_databases::AlbumDatabasesTable;
use crate::table::relations::album_share::AlbumShareTable;
use crate::table::relations::database_alias::DatabaseAliasTable;
use crate::table::relations::database_exif::DatabaseExifTable;
use crate::table::relations::tag_databases::TagDatabasesTable;
use rusqlite::Connection;

pub fn create_all_tables(conn: &Connection) -> rusqlite::Result<()> {
    // 1. 先建核心基礎表
    ObjectSchema::create_table(conn)?;

    // 2. 建專用元數據表
    ImageMetadataSchema::create_table(conn)?;
    VideoMetadataSchema::create_table(conn)?;
    AlbumMetadataSchema::create_table(conn)?; // 注意：這裡使用了新的 meta_album

    // 3. 建關聯表
    // 這些表通常透過 id 關聯，現在它們應該參照 object(id)
    AlbumDatabasesTable::create_table(conn)?;
    AlbumShareTable::create_table(conn)?;
    DatabaseAliasTable::create_table(conn)?;
    TagDatabasesTable::create_table(conn)?;
    DatabaseExifTable::create_table(conn)?; // Exif 表可以保持原樣，透過 ID 關聯到圖片

    Ok(())
}
