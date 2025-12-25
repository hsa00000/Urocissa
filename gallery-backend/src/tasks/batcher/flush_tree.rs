use mini_executor::BatchTask;

use crate::{
    public::{
        constant::redb::DATA_TABLE,
        db::tree::TREE,
        structure::abstract_data::AbstractData,
    },
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
    fn batch_run(list: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            let mut all_insert_data = Vec::new();
            let mut all_remove_abstract_data = Vec::new();
            for task in list {
                all_insert_data.extend(task.insert_list);
                all_remove_abstract_data.extend(task.remove_list);
            }
            flush_tree_task(all_insert_data, all_remove_abstract_data);
        }
    }
}

fn flush_tree_task(insert_list: Vec<AbstractData>, remove_list: Vec<AbstractData>) {
    let write_txn = TREE.in_disk.begin_write().unwrap();
    {
        let mut data_table = write_txn.open_table(DATA_TABLE).unwrap();

        insert_list
            .iter()
            .for_each(|abstract_data| {
                let hash = abstract_data.hash();
                data_table.insert(&*hash, abstract_data).unwrap();
            });
        remove_list
            .iter()
            .for_each(|abstract_data| {
                let hash = abstract_data.hash();
                data_table.remove(&*hash).unwrap();
            });
    };
    write_txn.commit().unwrap();
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
}
