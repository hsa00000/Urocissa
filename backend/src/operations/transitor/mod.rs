use crate::public::{
    db::tree_snapshot::read_tree_snapshot::MyCow,
    structure::{
        abstract_data::AbstractData,
        response::database_timestamp::DataBaseTimestampReturn,
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

pub fn hash_to_abstract_data(
    data_table: &ReadOnlyTable<&'static str, AbstractData>,
    hash: ArrayString<64>,
) -> Result<AbstractData> {
    if let Some(data) = data_table.get(&*hash)? {
        Ok(data.value())
    } else {
        Err(anyhow::anyhow!("No data found for hash: {}", hash))
    }
}

pub fn clear_abstract_data_metadata(abstract_data: &mut AbstractData, show_metadata: bool) {
    match abstract_data {
        AbstractData::Image(img) => {
            // Keep the original logic of reducing alias to the last item
            if let Some(last_alias) = img.metadata.alias.pop() {
                img.metadata.alias = vec![last_alias];
            } else {
                img.metadata.alias.clear();
            }

            if !show_metadata {
                img.metadata.albums.clear();
                img.object.tags.clear();
                img.metadata.alias.clear();
                img.metadata.exif_vec.clear();
            }
        }
        AbstractData::Video(vid) => {
            // Keep the original logic of reducing alias to the last item
            if let Some(last_alias) = vid.metadata.alias.pop() {
                vid.metadata.alias = vec![last_alias];
            } else {
                vid.metadata.alias.clear();
            }

            if !show_metadata {
                vid.metadata.albums.clear();
                vid.object.tags.clear();
                vid.metadata.alias.clear();
                vid.metadata.exif_vec.clear();
            }
        }
        AbstractData::Album(album) => {
            if !show_metadata {
                album.object.tags.clear();
            }
        }
    }
}

pub fn abstract_data_to_database_timestamp_return(
    mut abstract_data: AbstractData,
    timestamp: u128,
    show_download: bool,
    show_metadata: bool,
) -> DataBaseTimestampReturn {
    // Create the return object first (which computes timestamp from abstract_data)
    let result = DataBaseTimestampReturn::new(
        abstract_data.clone(),
        &crate::public::constant::DEFAULT_PRIORITY_LIST,
        timestamp,
        show_download,
    );

    // Then clear metadata from the abstract_data
    clear_abstract_data_metadata(&mut abstract_data, show_metadata);

    // Return with the cleared abstract_data but the original computed timestamp
    DataBaseTimestampReturn {
        abstract_data,
        timestamp: result.timestamp,
        token: result.token,
    }
}
