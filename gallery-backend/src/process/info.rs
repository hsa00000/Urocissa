use anyhow::{Context, Result};
use uuid::Uuid;

use crate::process::artifact_publisher::ArtifactPublisher;
use crate::process::media_pipeline::{MediaTaskPlan, ThumbnailPublishMode, execute_media_pipeline};
use crate::public::structure::abstract_data::AbstractData;

/// Run the same safe media plan used by selective reindex for a newly imported
/// image. Artifacts are staged beside their final path and promoted only after
/// every analysis stage succeeds.
pub fn process_image_info(abstract_data: &mut AbstractData) -> Result<()> {
    if !abstract_data.is_image() {
        anyhow::bail!("image pipeline received a non-image record");
    }
    process_safe_media_info(abstract_data).context("failed to process image info")
}

/// New videos initially run the safe metadata/thumbnail/hash plan. Their
/// pending record is published by `IndexTask` before `VideoTask` invokes the
/// shared video-compression stage.
pub fn process_video_info(abstract_data: &mut AbstractData) -> Result<()> {
    if !abstract_data.is_video() {
        anyhow::bail!("video pipeline received a non-video record");
    }
    process_safe_media_info(abstract_data).context("failed to process video info")
}

fn process_safe_media_info(abstract_data: &mut AbstractData) -> Result<()> {
    let token = format!("import-{}", Uuid::new_v4());
    let mut publisher = ArtifactPublisher::new(token);
    let result = execute_media_pipeline(
        abstract_data,
        &MediaTaskPlan::safe_default(),
        &mut publisher,
        ThumbnailPublishMode::Initial,
    )
    .map_err(anyhow::Error::new)?;
    publisher.publish(|| Ok(()))?;
    *abstract_data = result.candidate;
    Ok(())
}
