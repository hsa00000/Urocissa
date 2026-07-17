use mini_executor::BatchTask;

use crate::public::error_data::handle_error;
use crate::{
    public::{db::tree::TREE, structure::abstract_data::AbstractData},
    tasks::{BATCH_COORDINATOR, batcher::update_tree::UpdateTreeTask},
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

    pub fn remove(abstract_data_list: Vec<AbstractData>) -> Self {
        Self {
            insert_list: Vec::new(),
            remove_list: abstract_data_list,
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
    if let Err(error) = TREE.store.write(|data_table| {
        for abstract_data in insert_list {
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
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
}
