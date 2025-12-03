use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
use crate::workflow::tasks::batcher::flush_tree::FlushTreeTask;
use crate::table::image::ImageCombined;
use crate::table::object::ObjectSchema;
use crate::table::meta_image::ImageMetadataSchema;
use arrayvec::ArrayString;
use anyhow::Result;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashSet;

fn create_random_data() -> AbstractData {
    // 簡單的隨機數據生成 - 創建一個假的 ImageCombined
    let image = ImageCombined {
        object: ObjectSchema {
            id: ArrayString::from(&format!("random_{}", rand::random::<u64>())).unwrap(),
            obj_type: "image".to_string(),
            created_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64,
            pending: false,
            thumbhash: None,
        },
        metadata: ImageMetadataSchema {
            id: ArrayString::from(&format!("random_{}", rand::random::<u64>())).unwrap(),
            size: 1024,
            width: 100,
            height: 100,
            ext: "jpg".to_string(),
            phash: None,
        },
        albums: HashSet::new(),
    };
    
    AbstractData::Image(image)
}
#[get("/put/generate_random_data?<number>")]
pub async fn generate_random_data(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    number: usize,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let database_list: Vec<AbstractData> = (0..number)
        .into_par_iter()
        .map(|_| create_random_data())
        .collect();
    BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(database_list));
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update tree: {}", e))?;
    info!("Insert random data complete");
    Ok(())
}
