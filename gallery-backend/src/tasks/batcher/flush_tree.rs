use mini_executor::BatchTask;
use std::collections::{BTreeSet, HashSet};

use crate::public::error_data::handle_error;
use crate::public::{
    db::{
        tree::{TREE, state::TargetSet},
        write_behind::WRITE_BEHIND,
    },
    structure::abstract_data::AbstractData,
};

pub struct FlushTreeTask {
    pub insert_list: Vec<AbstractData>,
    pub remove_list: Vec<AbstractData>,
}

impl FlushTreeTask {
    pub fn insert(data_list: Vec<AbstractData>) -> Self {
        Self {
            insert_list: data_list,
            remove_list: Vec::new(),
        }
    }
}

impl BatchTask for FlushTreeTask {
    async fn batch_run(list: Vec<Self>) {
        let mut all_insert_data = Vec::new();
        let mut all_remove_abstract_data = Vec::new();
        for task in list {
            all_insert_data.extend(task.insert_list);
            all_remove_abstract_data.extend(task.remove_list);
        }
        flush_tree_task(&all_insert_data, &all_remove_abstract_data);
    }
}

fn flush_tree_task(insert_list: &[AbstractData], remove_list: &[AbstractData]) {
    let _persistence_guard = TREE.persistence_lock.lock().unwrap();
    let mut state = TREE.state.write().unwrap();
    let reconciled_insert_list = insert_list
        .iter()
        .map(|data| {
            let slot_ref = state.find(data.hash().as_str());
            WRITE_BEHIND
                .logical_record_for_slot(slot_ref, data.hash().as_str(), Some(data.clone()))
                .unwrap_or_else(|| data.clone())
        })
        .collect::<Vec<_>>();
    let reconciled_targets = TargetSet::from_slot_refs(
        insert_list
            .iter()
            .chain(remove_list)
            .filter_map(|data| state.find(data.hash().as_str())),
        state.arena.capacity(),
    );
    let reconciled_album_ids = insert_list
        .iter()
        .chain(remove_list)
        .filter(|data| matches!(data, AbstractData::Album(_)))
        .map(AbstractData::hash)
        .collect::<BTreeSet<_>>();
    if let Err(error) = TREE.store.write(|data_table| {
        for abstract_data in &reconciled_insert_list {
            data_table.insert(abstract_data)?;
        }
        for abstract_data in remove_list {
            let hash = abstract_data.hash();
            data_table.remove(hash.as_str())?;
        }
        Ok(())
    }) {
        handle_error(error);
        return;
    }
    WRITE_BEHIND.cancel_targets(&reconciled_targets, &reconciled_album_ids);
    let remove_ids = remove_list
        .iter()
        .map(AbstractData::hash)
        .collect::<HashSet<_>>();
    state.apply_batch(&reconciled_insert_list, &remove_ids);
    crate::public::db::tree::VERSION_COUNT_TIMESTAMP
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
