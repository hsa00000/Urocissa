use crate::operations::indexation::generate_dynamic_image::generate_dynamic_image;
use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::open_db::open_data_table;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::{AppResult, GuardResult};
use crate::tasks::batcher::flush_tree::FlushTreeTask;

use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::INDEX_COORDINATOR;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use arrayvec::ArrayString;
use rocket::form::{Errors, Form};
use rocket::fs::TempFile;

#[derive(FromForm, Debug)]
pub struct RegenerateThumbnailForm<'r> {
    /// Hash of the image to regenerate thumbnail for
    #[field(name = "hash")]
    pub hash: String,

    /// Frame file to use for thumbnail generation
    #[field(name = "frame")]
    pub frame: TempFile<'r>,
}

#[put("/put/regenerate-thumbnail-with-frame", data = "<form>")]
pub async fn regenerate_thumbnail_with_frame(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    form: Result<Form<RegenerateThumbnailForm<'_>>, Errors<'_>>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let mut inner_form = match form {
        Ok(form) => form.into_inner(),
        Err(errors) => {
            let error_chain = errors
                .iter()
                .map(|e| anyhow!(e.to_string()))
                .reduce(|acc, e| acc.context(e.to_string()));

            return match error_chain {
                Some(chain) => Err(chain.context("Failed to parse form").into()),
                None => Err(anyhow!("Failed to parse form with unknown error").into()),
            };
        }
    };

    // Convert hash string to ArrayString
    let hash = ArrayString::<64>::from(&inner_form.hash)
        .map_err(|_| anyhow!("Invalid hash length or format"))?;

    let file_path = format!("./object/compressed/{}/{}.jpg", &hash[0..2], hash.as_str());

    inner_form
        .frame
        .move_copy_to(&file_path)
        .await
        .context("Failed to copy frame file")?;

    let abstract_data = tokio::task::spawn_blocking(move || -> Result<AbstractData> {
        let data_table = open_data_table()?;
        let access_guard = data_table
            .get(&*hash)
            .context("Failed to fetch DB record")?
            .ok_or_else(|| anyhow!("Hash not found"))?;

        let mut database = access_guard.value();

        let dyn_img = generate_dynamic_image(&database).context("Failed to decode DynamicImage")?;

        database.thumbhash = generate_thumbhash(&dyn_img);
        database.phash = generate_phash(&dyn_img);

        Ok(database.into())
    })
    .await
    .context("Failed to spawn blocking task")??;

    INDEX_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(vec![abstract_data]))
        .await
        .context("Failed to execute FlushTreeTask")?;

    info!("Regenerating thumbnail successfully");
    Ok(())
}
