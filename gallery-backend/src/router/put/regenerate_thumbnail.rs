use crate::operations::indexation::generate_dynamic_image::generate_dynamic_image;
use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::open_db::open_data_table;
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::abstract_data::AbstractData;
use crate::router::{AppResult, GuardResult};
use crate::tasks::batcher::flush_tree::FlushTreeTask;

use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::INDEX_COORDINATOR;
use anyhow::Result;
use arrayvec::ArrayString;
use log::info;
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
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    form: Result<Form<RegenerateThumbnailForm<'_>>, Errors<'_>>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let mut inner_form = match form {
        Ok(form) => form.into_inner(),
        Err(errors) => {
            let error_msg = errors
                .iter()
                .fold(String::from("Form parsing failed: "), |acc, e| {
                    format!("{acc}; {e}")
                });
            return Err(AppError::new(ErrorKind::InvalidInput, error_msg));
        }
    };

    // Convert hash string to ArrayString
    let hash = ArrayString::<64>::from(&inner_form.hash)
        .map_err(|_| AppError::new(ErrorKind::InvalidInput, "Invalid hash length or format"))?;

    let root = crate::public::constant::storage::get_data_path();
    let file_path = root.join(format!(
        "object/compressed/{}/{}.jpg",
        &hash[0..2],
        hash.as_str()
    ));

    inner_form
        .frame
        .move_copy_to(&file_path)
        .await
        .or_raise(|| (ErrorKind::IO, "Failed to copy frame file"))?;

    let abstract_data = tokio::task::spawn_blocking(move || -> Result<AbstractData, AppError> {
        let data_table = open_data_table();
        let access_guard = data_table
            .get(&*hash)
            .or_raise(|| (ErrorKind::Database, "Failed to fetch DB record"))?
            .ok_or_else(|| AppError::new(ErrorKind::NotFound, "Hash not found"))?;

        let mut abstract_data = access_guard.into_value();

        let dyn_img = generate_dynamic_image(&abstract_data)
            .or_raise(|| (ErrorKind::Internal, "Failed to decode DynamicImage"))?;

        abstract_data.set_thumbhash(generate_thumbhash(&dyn_img));
        abstract_data.set_phash(generate_phash(&dyn_img));
        abstract_data.update_update_at();

        Ok(abstract_data)
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to spawn blocking task"))??;

    INDEX_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(vec![abstract_data]))
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to execute FlushTreeTask"))?;

    info!("Regenerating thumbnail successfully");
    Ok(())
}
