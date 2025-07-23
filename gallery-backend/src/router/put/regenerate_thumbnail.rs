use crate::operations::indexation::generate_dynamic_image::generate_dynamic_image;
use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::open_db::open_data_table;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::AppResult;
use crate::tasks::batcher::flush_tree::FlushTreeTask;

use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::INDEX_COORDINATOR;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use arrayvec::ArrayString;
use rocket::form::Form;
use rocket::form::{self, DataField, FromFormField, ValueField};
use rocket::fs::TempFile;
pub enum FrameData<'r> {
    Hash(ArrayString<64>),
    File(TempFile<'r>),
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for FrameData<'r> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let valid_hash = String::from_value(field)?;
        match ArrayString::<64>::from(&valid_hash) {
            Ok(hash) => Ok(FrameData::Hash(hash)),
            Err(_) => Err(form::Error::validation("Invalid hash length or format").into()),
        }
    }

    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        match TempFile::from_data(field).await {
            Ok(temp_file) => Ok(FrameData::File(temp_file)),
            Err(err) => Err(err),
        }
    }
}

#[put("/put/regenerate-thumbnail-with-frame", data = "<data>")]
pub async fn regenerate_thumbnail_with_frame(
    _auth: GuardAuth,
    _read_only_mode: GuardReadOnlyMode,
    data: Form<Vec<FrameData<'_>>>,
) -> AppResult<()> {
    let mut hash: Option<ArrayString<64>> = None;

    for frame_data in data.into_inner() {
        match frame_data {
            FrameData::Hash(received_hash) => hash = Some(received_hash),
            FrameData::File(mut file) => {
                let hash = hash
                    .clone()
                    .ok_or_else(|| anyhow!("Missing hash before frame file"))?;

                let file_path =
                    format!("./object/compressed/{}/{}.jpg", &hash[0..2], hash.as_str());

                file.move_copy_to(&file_path)
                    .await
                    .context("Copy frame file failed")?;

                let abstract_data = tokio::task::spawn_blocking(move || -> Result<AbstractData> {
                    let data_table = open_data_table().context("Open DATA_TABLE failed")?;
                    let access_guard = data_table
                        .get(&*hash)
                        .context("Fetch DB record failed")?
                        .ok_or_else(|| anyhow!("Hash not found in DB"))?;

                    let mut database = access_guard.value();

                    let dyn_img =
                        generate_dynamic_image(&database).context("Decode DynamicImage failed")?;

                    database.thumbhash = generate_thumbhash(&dyn_img);
                    database.phash = generate_phash(&dyn_img);

                    Ok(AbstractData::Database(database))
                })
                .await
                .context("Spawn-blocking join failed")??;

                INDEX_COORDINATOR
                    .execute_batch_waiting(FlushTreeTask::insert(vec![abstract_data]))
                    .await
                    .context("FlushTreeTask failed")?;
            }
        }
    }

    info!("Regenerating thumbnail successfully");
    Ok(())
}
