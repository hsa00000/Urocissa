use crate::public::{
    db::tree_snapshot::read_tree_snapshot::MyCow,
    structure::{
        abstract_data::AbstractData, database_struct::database_timestamp::DataBaseTimestampReturn,
    },
};
use anyhow::Result;
use arrayvec::ArrayString;

pub fn index_to_hash(tree_snapshot: &MyCow, index: usize) -> Result<ArrayString<64>> {
    if index >= tree_snapshot.len() {
        return Err(anyhow::anyhow!("Index out of bounds: {}", index));
    }
    let hash = tree_snapshot.get_hash(index)?;
    Ok(hash)
}

pub fn clear_abstract_data_metadata(abstract_data: &mut AbstractData, show_metadata: bool) {
    match abstract_data {
        AbstractData::Database(database) => {
            database.alias = vec![database.alias.pop().unwrap()];
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
