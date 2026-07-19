use anyhow::{Context, Result};
use arrayvec::ArrayString;
use image::ImageFormat;
use log::info;
use rocket::form::{Errors, Form};
use rocket::fs::TempFile;
use uuid::Uuid;

use crate::operations::indexation::fix_orientation::fix_video_width_height;
use crate::operations::indexation::generate_dynamic_image::decode_image;
use crate::operations::indexation::generate_image_hash::generate_thumbhash;
use crate::operations::indexation::generate_width_height::generate_video_width_height;
use crate::process::artifact_publisher::ArtifactPublisher;
use crate::process::media_lock::lock_media;
use crate::process::media_publish::{load_logical_media, publish_media_mutation};
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};

#[derive(FromForm, Debug)]
pub struct RegenerateThumbnailForm<'r> {
    #[field(name = "hash")]
    pub hash: String,
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
    let mut form = form.map_err(|errors| {
        AppError::new(
            ErrorKind::InvalidInput,
            errors
                .iter()
                .fold("Form parsing failed".to_string(), |message, error| {
                    format!("{message}; {error}")
                }),
        )
    })?;
    let hash = ArrayString::<64>::from(&form.hash)
        .map_err(|_| AppError::new(ErrorKind::InvalidInput, "invalid object id"))?;
    let _media_guard = lock_media(hash).await;

    let root = crate::public::constant::storage::get_data_path();
    let destination = root.join(format!(
        "object/compressed/{}/{}.jpg",
        &hash[0..2],
        hash.as_str()
    ));
    let mut publisher = ArtifactPublisher::new(format!("capture-{}", Uuid::new_v4()));
    let staged = publisher
        .stage_path(&destination)
        .map_err(|error| AppError::from_err(ErrorKind::IO, error))?;
    form.frame
        .move_copy_to(&staged)
        .await
        .or_raise(|| (ErrorKind::IO, "failed to stage captured video frame"))?;
    publisher.replace(staged.clone(), destination);

    tokio::task::spawn_blocking(move || capture_frame_inner(hash, &staged, publisher))
        .await
        .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))?
        .map_err(|error| AppError::from_err(ErrorKind::IO, error))?;
    info!("Video frame thumbnail regenerated successfully");
    Ok(())
}

fn capture_frame_inner(
    hash: ArrayString<64>,
    staged: &std::path::Path,
    publisher: ArtifactPublisher,
) -> Result<()> {
    let (slot_ref, mut data) = load_logical_media(hash)?;
    if !data.is_video() {
        anyhow::bail!("captured-frame thumbnails only apply to videos");
    }
    let image = decode_image(staged).context("failed to decode captured video frame")?;
    image
        .to_rgb8()
        .save_with_format(staged, ImageFormat::Jpeg)
        .context("failed to normalize captured frame as JPEG")?;
    let thumbhash = generate_thumbhash(&image);
    let (width, height) = generate_video_width_height(&data)?;
    data.set_width(width);
    data.set_height(height);
    fix_video_width_height(&mut data);
    let (width, height) = (data.width(), data.height());

    publish_media_mutation(slot_ref, hash, publisher, move |latest| {
        let AbstractData::Video(video) = latest else {
            anyhow::bail!("media type changed before captured frame was published");
        };
        video.metadata.width = width;
        video.metadata.height = height;
        video.object.thumbhash = Some(thumbhash);
        Ok(())
    })?;
    Ok(())
}
