use log::error;
use mini_executor::BatchTask;
use serde_json;

use crate::{public::db::tree::TREE, public::structure::abstract_data::AbstractData};

pub struct UpdateTagsTask {
    pub updates: Vec<AbstractData>,
}

impl UpdateTagsTask {
    pub fn new(updates: Vec<AbstractData>) -> Self {
        Self { updates }
    }
}

impl BatchTask for UpdateTagsTask {
    fn batch_run(list: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            let mut all_updates = Vec::new();
            for task in list {
                all_updates.extend(task.updates);
            }
            if let Err(e) = update_tags_task(all_updates) {
                error!("Error in update_tags_task: {}", e);
            }
        }
    }
}

fn update_tags_task(updates: Vec<AbstractData>) -> rusqlite::Result<()> {
    let conn = TREE.get_connection().unwrap();

    for abstract_data in updates {
        match abstract_data {
            AbstractData::Database(_) => {}
            AbstractData::Album(album) => {
                // Update the tag column with new JSON
                conn.execute(
                    "UPDATE album SET tag = ? WHERE id = ?",
                    rusqlite::params![
                        serde_json::to_string(&album.tag.iter().collect::<Vec<_>>()).unwrap(),
                        album.id.as_str()
                    ],
                )?;
            }
        }
    }

    Ok(())
}
