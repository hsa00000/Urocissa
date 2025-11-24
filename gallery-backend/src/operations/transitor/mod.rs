use crate::public::{
    db::tree_snapshot::read_tree_snapshot::MyCow, structure::abstract_data::AbstractData,
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
            if !show_metadata {
                database.album.clear();
            }
        }
        AbstractData::Album(album) => {
            if !show_metadata {
                album.tag.clear();
            }
        }
    }
}

pub fn process_abstract_data_for_response(
    mut abstract_data: AbstractData,
    show_metadata: bool,
) -> AbstractData {
    clear_abstract_data_metadata(&mut abstract_data, show_metadata);
    abstract_data
}
