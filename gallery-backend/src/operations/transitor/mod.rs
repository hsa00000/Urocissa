use crate::public::{
    db::tree_snapshot::read_tree_snapshot::MyCow,
    structure::{
        abstract_data::AbstractData,
        album::Album,
        database_struct::{
            database::definition::Database, database_timestamp::DataBaseTimestampReturn,
        },
    },
};
use anyhow::Result;
use arrayvec::ArrayString;
use redb::ReadOnlyTable;

pub fn index_to_hash(tree_snapshot: &MyCow, index: usize) -> Result<ArrayString<64>> {
    if index >= tree_snapshot.len() {
        return Err(anyhow::anyhow!("Index out of bounds: {}", index));
    }
    let hash = tree_snapshot.get_hash(index)?;
    Ok(hash)
}

pub fn hash_to_database(
    data_table: &ReadOnlyTable<&'static str, Database>,
    hash: ArrayString<64>,
) -> Result<Database> {
    if let Some(database) = data_table.get(&*hash)? {
        let database = database.value();
        Ok(database)
    } else {
        Err(anyhow::anyhow!("No data found for hash: {}", hash))
    }
}

pub fn hash_to_album(
    album_table: &ReadOnlyTable<&'static str, Album>,
    hash: ArrayString<64>,
) -> Result<Album> {
    if let Some(album) = album_table.get(&*hash)? {
        let album = album.value();
        Ok(album)
    } else {
        Err(anyhow::anyhow!("No album found for hash: {}", hash))
    }
}

pub fn hash_to_abstract_data(
    data_table: &ReadOnlyTable<&'static str, Database>,
    album_table: &ReadOnlyTable<&'static str, Album>,
    hash: ArrayString<64>,
) -> Result<AbstractData> {
    if let Some(database) = data_table.get(&*hash)? {
        let database = database.value();
        Ok(database.into())
    } else if let Some(album) = album_table.get(&*hash)? {
        let album = album.value();
        Ok(album.into())
    } else {
        Err(anyhow::anyhow!("No data found for hash: {}", hash))
    }
}

pub fn clear_abstract_data_metadata(abstract_data: &mut AbstractData, show_metadata: bool) {
    match abstract_data {
        AbstractData::Database(database) => {
            if !show_metadata {
                database.tag.clear();
                database.album.clear();
                database.alias.clear();
            }
        }
        AbstractData::Album(album) => {
            if !show_metadata {
                album.tag.clear();
            }
        }
    }
}

pub fn abstract_data_to_database_timestamp_return(
    abstract_data: AbstractData,
    timestamp: u128,
    show_download: bool,
) -> DataBaseTimestampReturn {
    DataBaseTimestampReturn::new(
        abstract_data,
        &crate::public::constant::DEFAULT_PRIORITY_LIST,
        timestamp,
        show_download,
    )
}
