use mini_executor::BatchTask;

use crate::{
    public::{
        constant::redb::{ALBUM_TABLE, DATA_TABLE},
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
    pub fn insert(databases: Vec<AbstractData>) -> Self {
        Self {
            insert_list: databases,
            remove_list: Vec::new(),
        }
    }
    pub fn remove(databases: Vec<AbstractData>) -> Self {
        Self {
            insert_list: Vec::new(),
            remove_list: databases,
        }
    }
}
impl BatchTask for FlushTreeTask {
    fn batch_run(list: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            let mut all_insert_databases = Vec::new();
            let mut all_remove_databases = Vec::new();
            for task in list {
                all_insert_databases.extend(task.insert_list);
                all_remove_databases.extend(task.remove_list);
            }
            flush_tree_task(all_insert_databases, all_remove_databases);
        }
    }
}

fn flush_tree_task(insert_list: Vec<AbstractData>, remove_list: Vec<AbstractData>) {
    let write_txn = TREE.in_disk.begin_write().unwrap();
    {
        let mut data_table = write_txn.open_table(DATA_TABLE).unwrap();
        let mut album_table = write_txn.open_table(ALBUM_TABLE).unwrap();

        insert_list
            .iter()
            .for_each(|abstract_data| match abstract_data {
                AbstractData::Database(database) => {
                    data_table.insert(&*database.hash, database).unwrap();
                }
                AbstractData::Album(album) => {
                    album_table.insert(&*album.id, album).unwrap();
                }
            });
        remove_list
            .iter()
            .for_each(|abstract_data| match abstract_data {
                AbstractData::Database(database) => {
                    data_table.remove(&*database.hash).unwrap();
                }
                AbstractData::Album(album) => {
                    album_table.remove(&*album.id).unwrap();
                }
            });
    };
    write_txn.commit().unwrap();
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
}
