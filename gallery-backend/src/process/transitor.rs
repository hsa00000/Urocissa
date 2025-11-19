use crate::{
    operations::transitor::{
        hash_to_abstract_data, hash_to_database, index_to_hash,
    },
    public::{
        db::tree_snapshot::read_tree_snapshot::MyCow,
        structure::{
            database_struct::database::definition::Database,
        },
    },
};

use anyhow::{Result, anyhow};

pub fn index_to_database(
    tree_snapshot: &MyCow,
    index: usize,
) -> Result<Database> {
    let hash = index_to_hash(&tree_snapshot, index)
        .map_err(|e| anyhow!("Failed to read hash by index {}: {}", index, e))?;
    let data = hash_to_database(hash)
        .map_err(|e| anyhow!("Failed to read database by hash {}: {}", hash, e))?;
    Ok(data)
}

pub fn index_to_abstract_data(
    tree_snapshot: &MyCow,
    index: usize,
) -> Result<crate::public::structure::abstract_data::AbstractData> {
    let hash = index_to_hash(&tree_snapshot, index)
        .map_err(|e| anyhow!("Failed to read hash by index {}: {}", index, e))?;
    let abstract_data = hash_to_abstract_data(hash)
        .map_err(|e| anyhow!("Failed to read abstract data by hash {}: {}", hash, e))?;
    Ok(abstract_data)
}

