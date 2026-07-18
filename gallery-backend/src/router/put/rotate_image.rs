use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::indexation::generate_thumbnail::generate_thumbnail_for_image;
use crate::operations::open_db::open_data_table;
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::abstract_data::AbstractData;
use crate::router::{AppResult, GuardResult};
use crate::tasks::batcher::flush_tree::FlushTreeTask;

use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::INDEX_COORDINATOR;
use anyhow::Result;
// use anyhow::anyhow;
use arrayvec::ArrayString;
use log::info;
use rocket::serde::{Deserialize, json::Json};

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct RotateImageRequest {
    /// Hash of the image to rotate
    pub hash: String,
}

#[put("/put/rotate-image", data = "<request>")]
pub async fn rotate_image(
    auth: GuardResult<GuardAuth>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    request: Json<RotateImageRequest>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;

    // Convert hash string to ArrayString
    let hash = ArrayString::<64>::from(&request.hash)
        .map_err(|_| AppError::new(ErrorKind::InvalidInput, "Invalid hash length or format"))?;

    let abstract_data =
        tokio::task::spawn_blocking(move || -> Result<Vec<AbstractData>, AppError> {
            let data_table = open_data_table();
            let access_guard = data_table
                .get(&*hash)
                .or_raise(|| (ErrorKind::Database, "Failed to fetch DB record"))?
                .ok_or_else(|| AppError::new(ErrorKind::NotFound, "Hash not found"))?;

            let mut abstract_data = access_guard.into_value();

            // Only rotate images, not videos or albums
            if !matches!(abstract_data, AbstractData::Image(_)) {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "Only images can be rotated",
                ));
            }

            // Load the compressed image (not the original)
            let compressed_path = abstract_data.compressed_path();
            let mut dyn_img = image::open(&compressed_path).map_err(|e| {
                AppError::new(
                    ErrorKind::IO,
                    format!(
                        "Failed to load compressed image: {} ({e})",
                        compressed_path.display()
                    ),
                )
            })?;

            // Rotate counter-clockwise (270 degrees clockwise = 90 degrees counter-clockwise)
            dyn_img = dyn_img.rotate270();

            // Swap width and height after rotation
            abstract_data.swap_width_height();

            // Generate and save the rotated thumbnail
            generate_thumbnail_for_image(&mut abstract_data, &dyn_img.clone()).or_raise(|| {
                (
                    ErrorKind::Internal,
                    "Failed to generate thumbnail for rotated image",
                )
            })?;

            // Update thumbhash and phash with the rotated image
            abstract_data.set_thumbhash(generate_thumbhash(&dyn_img));
            abstract_data.set_phash(generate_phash(&dyn_img));
            abstract_data.update_update_at();

            let album_ids: Vec<_> = abstract_data
                .albums()
                .map(|albums| albums.iter().copied().collect())
                .unwrap_or_default();

            let mut result_vec = vec![abstract_data];

            for album_id in album_ids {
                if let Ok(Some(access_guard)) = data_table.get(album_id.as_str()) {
                    let mut album = access_guard.into_value();
                    album.update_update_at();
                    result_vec.push(album);
                }
            }

            Ok(result_vec)
        })
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to spawn blocking task"))??;

    INDEX_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(abstract_data))
        .await
        .or_raise(|| (ErrorKind::Internal, "Failed to execute FlushTreeTask"))?;

    info!("Image rotated successfully");
    Ok(())
}
