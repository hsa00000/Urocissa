use crate::{
    operations::transitor::{hash_to_abstract_data, index_to_hash},
    public::{
        db::tree_snapshot::read_tree_snapshot::MyCow,
        structure::abstract_data::AbstractData,
    },
};
use anyhow::{Result, anyhow};
use redb::ReadOnlyTable;

pub fn index_to_abstract_data(
    tree_snapshot: &MyCow,
    data_table: &ReadOnlyTable<&'static str, AbstractData>,
    index: usize,
) -> Result<AbstractData> {
    let hash = index_to_hash(&tree_snapshot, index)
        .map_err(|e| anyhow!("Failed to read hash by index {}: {}", index, e))?;
    let abstract_data = hash_to_abstract_data(data_table, hash)
        .map_err(|e| anyhow!("Failed to read abstract data by hash {}: {}", hash, e))?;
    Ok(abstract_data)
}
