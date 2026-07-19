use anyhow::{Context, Result};
use arrayvec::ArrayString;
use log::info;
use rocket::serde::{Deserialize, json::Json};
use uuid::Uuid;

use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::indexation::generate_thumbnail::generate_thumbnail_for_image_to;
use crate::process::artifact_publisher::ArtifactPublisher;
use crate::process::media_lock::lock_media;
use crate::process::media_publish::{load_logical_media, publish_media_mutation};
use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct RotateImageRequest {
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
    let hash = ArrayString::<64>::from(&request.hash)
        .map_err(|_| AppError::new(ErrorKind::InvalidInput, "invalid object id"))?;
    let _media_guard = lock_media(hash).await;
    tokio::task::spawn_blocking(move || rotate_image_inner(hash))
        .await
        .map_err(|error| AppError::from_err(ErrorKind::Internal, error.into()))?
        .map_err(|error| AppError::from_err(ErrorKind::IO, error))?;
    info!("Image rotated successfully");
    Ok(())
}

fn rotate_image_inner(hash: ArrayString<64>) -> Result<()> {
    let (slot_ref, mut data) = load_logical_media(hash)?;
    if !data.is_image() {
        anyhow::bail!("only images can be rotated");
    }
    let compressed_path = data.compressed_path();
    let rotated = image::open(&compressed_path)
        .with_context(|| format!("failed to load {}", compressed_path.display()))?
        .rotate270();
    data.swap_width_height();
    let width = data.width();
    let height = data.height();
    let thumbhash = generate_thumbhash(&rotated);
    let phash = generate_phash(&rotated);

    let mut publisher = ArtifactPublisher::new(format!("rotate-{}", Uuid::new_v4()));
    let staged = publisher.stage_path(&compressed_path)?;
    generate_thumbnail_for_image_to(&data, &rotated, &staged)?;
    publisher.replace(staged, compressed_path);
    publish_media_mutation(slot_ref, hash, publisher, move |latest| {
        let AbstractData::Image(image) = latest else {
            anyhow::bail!("media type changed before image rotation was published");
        };
        image.metadata.width = width;
        image.metadata.height = height;
        image.object.thumbhash = Some(thumbhash);
        image.metadata.phash = Some(phash);
        Ok(())
    })?;
    Ok(())
}
