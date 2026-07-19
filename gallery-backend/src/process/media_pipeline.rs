use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::operations::indexation::fix_orientation::{
    fix_image_orientation, fix_video_width_height,
};
use crate::operations::indexation::generate_compressed_video::{
    VideoCompressionOutcome, generate_compressed_video_to, is_static_gif,
};
use crate::operations::indexation::generate_dynamic_image::decode_image;
use crate::operations::indexation::generate_exif::{
    generate_exif_for_image, generate_exif_for_video,
};
use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::indexation::generate_thumbnail::{
    generate_thumbnail_for_image_to, generate_thumbnail_for_video_to,
};
use crate::operations::indexation::generate_width_height::{
    generate_image_width_height, generate_video_width_height,
};
use crate::process::artifact_publisher::ArtifactPublisher;
use crate::public::structure::abstract_data::AbstractData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReindexOperation {
    Exif,
    Dimensions,
    FileSize,
    Thumbnail,
    VisualHashes,
    VideoCompression,
    ClearTags,
}

impl ReindexOperation {
    pub const ALL: [Self; 7] = [
        Self::Exif,
        Self::Dimensions,
        Self::FileSize,
        Self::Thumbnail,
        Self::VisualHashes,
        Self::VideoCompression,
        Self::ClearTags,
    ];

    pub const SAFE_DEFAULT: [Self; 5] = [
        Self::Exif,
        Self::Dimensions,
        Self::FileSize,
        Self::Thumbnail,
        Self::VisualHashes,
    ];

    pub const fn applies_to(self, is_image: bool) -> bool {
        !matches!(self, Self::VideoCompression) || !is_image
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaStage {
    FileSize,
    MetadataProbe,
    DecodeAndDimensions,
    Thumbnail,
    VisualHashes,
    VideoCompression,
    ClearTags,
    Publish,
}

impl MediaStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileSize => "fileSize",
            Self::MetadataProbe => "metadata",
            Self::DecodeAndDimensions => "dimensions",
            Self::Thumbnail => "thumbnail",
            Self::VisualHashes => "visualHashes",
            Self::VideoCompression => "videoCompression",
            Self::ClearTags => "clearTags",
            Self::Publish => "publish",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaTaskPlan {
    operations: Vec<ReindexOperation>,
}

impl MediaTaskPlan {
    pub fn new(operations: Vec<ReindexOperation>) -> Result<Self> {
        if operations.is_empty() {
            anyhow::bail!("at least one reindex operation is required");
        }
        let requested = operations.into_iter().collect::<HashSet<_>>();
        let operations = ReindexOperation::ALL
            .into_iter()
            .filter(|operation| requested.contains(operation))
            .collect();
        Ok(Self { operations })
    }

    pub fn safe_default() -> Self {
        Self {
            operations: ReindexOperation::SAFE_DEFAULT.to_vec(),
        }
    }

    pub fn operations(&self) -> &[ReindexOperation] {
        &self.operations
    }

    pub fn contains(&self, operation: ReindexOperation) -> bool {
        self.operations.contains(&operation)
    }

    pub fn has_applicable_operation(&self, data: &AbstractData) -> bool {
        !matches!(data, AbstractData::Album(_))
            && self
                .operations
                .iter()
                .any(|operation| operation.applies_to(data.is_image()))
    }

    pub fn stages_for(&self, data: &AbstractData) -> Vec<MediaStage> {
        let mut stages = Vec::new();
        if self.contains(ReindexOperation::FileSize) {
            stages.push(MediaStage::FileSize);
        }
        let derived = self.contains(ReindexOperation::Dimensions)
            || self.contains(ReindexOperation::Thumbnail)
            || self.contains(ReindexOperation::VisualHashes);
        if self.contains(ReindexOperation::Exif)
            || derived
            || data.is_video() && self.contains(ReindexOperation::VideoCompression)
        {
            stages.push(MediaStage::MetadataProbe);
        }
        if derived {
            stages.push(MediaStage::DecodeAndDimensions);
        }
        if self.contains(ReindexOperation::Thumbnail) {
            stages.push(MediaStage::Thumbnail);
        }
        if self.contains(ReindexOperation::VisualHashes) {
            stages.push(MediaStage::VisualHashes);
        }
        if data.is_video() && self.contains(ReindexOperation::VideoCompression) {
            stages.push(MediaStage::VideoCompression);
        }
        if self.contains(ReindexOperation::ClearTags) {
            stages.push(MediaStage::ClearTags);
        }
        stages.push(MediaStage::Publish);
        stages
    }
}

#[derive(Debug)]
pub struct MediaPipelineError {
    pub stage: MediaStage,
    pub source: anyhow::Error,
}

impl fmt::Display for MediaPipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {:#}", self.stage.as_str(), self.source)
    }
}

impl std::error::Error for MediaPipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[derive(Debug)]
pub struct MediaPipelineResult {
    pub candidate: AbstractData,
    pub static_gif_conversion: bool,
}

/// Apply only user-selected durable outputs to a freshly rehydrated logical
/// record. Static GIF conversion is the documented exception and replaces all
/// image-analysis fields while retaining concurrent user metadata.
pub fn apply_selected_outputs(
    latest: &mut AbstractData,
    result: &MediaPipelineResult,
    plan: &MediaTaskPlan,
) -> Result<()> {
    let candidate = &result.candidate;
    if result.static_gif_conversion {
        latest.convert_to_image();
        let (AbstractData::Image(latest_image), AbstractData::Image(candidate_image)) =
            (&mut *latest, candidate)
        else {
            anyhow::bail!("static GIF pipeline did not produce an image record");
        };
        latest_image.metadata.size = candidate_image.metadata.size;
        latest_image.metadata.width = candidate_image.metadata.width;
        latest_image.metadata.height = candidate_image.metadata.height;
        latest_image.metadata.exif_vec = candidate_image.metadata.exif_vec.clone();
        latest_image
            .metadata
            .phash
            .clone_from(&candidate_image.metadata.phash);
        latest_image
            .object
            .thumbhash
            .clone_from(&candidate_image.object.thumbhash);
        latest_image.object.pending = false;
    } else {
        if plan.contains(ReindexOperation::FileSize) {
            match (&mut *latest, candidate) {
                (AbstractData::Image(current), AbstractData::Image(next)) => {
                    current.metadata.size = next.metadata.size;
                }
                (AbstractData::Video(current), AbstractData::Video(next)) => {
                    current.metadata.size = next.metadata.size;
                }
                _ => anyhow::bail!("media type changed while applying file size"),
            }
        }
        if plan.contains(ReindexOperation::Exif) {
            let next_exif = candidate
                .exif_vec()
                .cloned()
                .context("pipeline candidate has no EXIF map")?;
            let current_exif = latest
                .exif_vec_mut()
                .context("latest record has no EXIF map")?;
            *current_exif = next_exif;
        }
        if plan.contains(ReindexOperation::Dimensions) {
            latest.set_width(candidate.width());
            latest.set_height(candidate.height());
        }
        if plan.contains(ReindexOperation::VisualHashes) {
            match (&mut *latest, candidate) {
                (AbstractData::Image(current), AbstractData::Image(next)) => {
                    current.object.thumbhash.clone_from(&next.object.thumbhash);
                    current.metadata.phash.clone_from(&next.metadata.phash);
                }
                (AbstractData::Video(current), AbstractData::Video(next)) => {
                    current.object.thumbhash.clone_from(&next.object.thumbhash);
                }
                _ => anyhow::bail!("media type changed while applying visual hashes"),
            }
        }
        if plan.contains(ReindexOperation::VideoCompression) && latest.is_video() {
            latest.set_pending(false);
        }
    }

    if plan.contains(ReindexOperation::ClearTags) {
        latest.tag_mut().clear();
    }
    Ok(())
}

pub fn execute_media_pipeline(
    data: &AbstractData,
    plan: &MediaTaskPlan,
    publisher: &mut ArtifactPublisher,
) -> std::result::Result<MediaPipelineResult, MediaPipelineError> {
    execute_media_pipeline_with_video_progress(data, plan, publisher, false)
}

pub fn execute_media_pipeline_with_video_progress(
    data: &AbstractData,
    plan: &MediaTaskPlan,
    publisher: &mut ArtifactPublisher,
    report_video_progress: bool,
) -> std::result::Result<MediaPipelineResult, MediaPipelineError> {
    let mut candidate = data.clone();
    if plan.contains(ReindexOperation::FileSize) {
        let size = stage(
            MediaStage::FileSize,
            fs::metadata(candidate.imported_path())
                .context("failed to read canonical imported file metadata")
                .map(|metadata| metadata.len()),
        )?;
        candidate.set_size(size);
    }

    if candidate.is_video()
        && plan.contains(ReindexOperation::VideoCompression)
        && stage(MediaStage::MetadataProbe, is_static_gif(&candidate))?
    {
        candidate.convert_to_image();
        let size = stage(
            MediaStage::FileSize,
            fs::metadata(candidate.imported_path())
                .context("failed to read static GIF imported file metadata")
                .map(|metadata| metadata.len()),
        )?;
        candidate.set_size(size);
        run_image_pipeline(&mut candidate, &MediaTaskPlan::safe_default(), publisher)?;
        candidate.set_pending(false);
        publisher.remove(video_compressed_path(data));
        return Ok(MediaPipelineResult {
            candidate,
            static_gif_conversion: true,
        });
    }

    if candidate.is_image() {
        run_image_pipeline(&mut candidate, plan, publisher)?;
    } else if candidate.is_video() {
        run_video_pipeline(&mut candidate, plan, publisher, report_video_progress)?;
    }

    Ok(MediaPipelineResult {
        candidate,
        static_gif_conversion: false,
    })
}

fn run_image_pipeline(
    candidate: &mut AbstractData,
    plan: &MediaTaskPlan,
    publisher: &mut ArtifactPublisher,
) -> std::result::Result<(), MediaPipelineError> {
    let needs_visual_input = plan.contains(ReindexOperation::Dimensions)
        || plan.contains(ReindexOperation::Thumbnail)
        || plan.contains(ReindexOperation::VisualHashes);
    if plan.contains(ReindexOperation::Exif) || needs_visual_input {
        let exif = generate_exif_for_image(candidate);
        if let Some(current) = candidate.exif_vec_mut() {
            *current = exif;
        }
    }

    let mut decoded = if needs_visual_input {
        Some(stage(
            MediaStage::DecodeAndDimensions,
            decode_image(&candidate.imported_path()).context("failed to decode imported image"),
        )?)
    } else {
        None
    };
    if let Some(image) = decoded.as_mut() {
        fix_image_orientation(candidate, image);
        let (width, height) = generate_image_width_height(image);
        candidate.set_width(width);
        candidate.set_height(height);
    }

    let staged_thumbnail = if plan.contains(ReindexOperation::Thumbnail) {
        let destination = candidate.compressed_path();
        let staged = stage(MediaStage::Thumbnail, publisher.stage_path(&destination))?;
        let image = decoded
            .as_ref()
            .expect("thumbnail operation must decode the imported image");
        stage(
            MediaStage::Thumbnail,
            generate_thumbnail_for_image_to(candidate, image, &staged),
        )?;
        publisher.replace(staged.clone(), destination);
        Some(staged)
    } else {
        None
    };

    if plan.contains(ReindexOperation::VisualHashes) {
        let existing_thumbnail = candidate.compressed_path();
        let hash_source = staged_thumbnail.as_deref().unwrap_or(&existing_thumbnail);
        let image = stage(
            MediaStage::VisualHashes,
            decode_image(hash_source).context("failed to decode image thumbnail for hashing"),
        )?;
        candidate.set_thumbhash(generate_thumbhash(&image));
        candidate.set_phash(generate_phash(&image));
    }
    Ok(())
}

fn run_video_pipeline(
    candidate: &mut AbstractData,
    plan: &MediaTaskPlan,
    publisher: &mut ArtifactPublisher,
    report_video_progress: bool,
) -> std::result::Result<(), MediaPipelineError> {
    let needs_probe = plan.contains(ReindexOperation::Exif)
        || plan.contains(ReindexOperation::Dimensions)
        || plan.contains(ReindexOperation::Thumbnail)
        || plan.contains(ReindexOperation::VideoCompression);
    if needs_probe {
        let exif = stage(
            MediaStage::MetadataProbe,
            generate_exif_for_video(candidate),
        )?;
        if let Some(current) = candidate.exif_vec_mut() {
            *current = exif;
        }
    }

    if needs_probe {
        let (width, height) = stage(
            MediaStage::DecodeAndDimensions,
            generate_video_width_height(candidate),
        )?;
        candidate.set_width(width);
        candidate.set_height(height);
        fix_video_width_height(candidate);
    }

    let staged_thumbnail = if plan.contains(ReindexOperation::Thumbnail) {
        let destination = PathBuf::from(candidate.thumbnail_path());
        let staged = stage(MediaStage::Thumbnail, publisher.stage_path(&destination))?;
        stage(
            MediaStage::Thumbnail,
            generate_thumbnail_for_video_to(candidate, &staged),
        )?;
        publisher.replace(staged.clone(), destination);
        Some(staged)
    } else {
        None
    };

    if plan.contains(ReindexOperation::VisualHashes) {
        let existing_thumbnail = PathBuf::from(candidate.thumbnail_path());
        let hash_source = staged_thumbnail.as_deref().unwrap_or(&existing_thumbnail);
        let image = stage(
            MediaStage::VisualHashes,
            decode_image(hash_source).context("failed to decode video thumbnail for hashing"),
        )?;
        candidate.set_thumbhash(generate_thumbhash(&image));
    }

    if plan.contains(ReindexOperation::VideoCompression) {
        let destination = candidate.compressed_path();
        let staged = stage(
            MediaStage::VideoCompression,
            publisher.stage_path(&destination),
        )?;
        let outcome = stage(
            MediaStage::VideoCompression,
            generate_compressed_video_to(candidate, &staged, report_video_progress),
        )?;
        debug_assert_eq!(outcome, VideoCompressionOutcome::Video);
        publisher.replace(staged, destination);
        candidate.set_pending(false);
    }
    Ok(())
}

fn video_compressed_path(data: &AbstractData) -> PathBuf {
    let hash = data.hash();
    crate::public::constant::storage::get_data_path().join(format!(
        "object/compressed/{}/{}.mp4",
        &hash.as_str()[0..2],
        hash
    ))
}

fn stage<T>(stage: MediaStage, result: Result<T>) -> std::result::Result<T, MediaPipelineError> {
    result.map_err(|source| MediaPipelineError { stage, source })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        MediaPipelineResult, MediaStage, MediaTaskPlan, ReindexOperation, apply_selected_outputs,
    };
    use crate::public::structure::abstract_data::AbstractData;
    use crate::public::structure::image::{ImageCombined, ImageMetadata};
    use crate::public::structure::object::{ObjectSchema, ObjectType};
    use crate::public::structure::video::{VideoCombined, VideoMetadata};

    fn hash() -> arrayvec::ArrayString<64> {
        arrayvec::ArrayString::from(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap()
    }

    fn image() -> AbstractData {
        let hash = hash();
        AbstractData::Image(ImageCombined {
            object: ObjectSchema::new(hash, ObjectType::Image),
            metadata: ImageMetadata::new(hash, 10, 20, 30, "jpg".to_string()),
        })
    }

    fn video() -> AbstractData {
        let hash = hash();
        AbstractData::Video(VideoCombined {
            object: ObjectSchema::new(hash, ObjectType::Video),
            metadata: VideoMetadata::new(hash, 10, 20, 30, "mp4".to_string()),
        })
    }

    #[test]
    fn operation_wire_format_is_camel_case() {
        assert_eq!(
            serde_json::to_string(&ReindexOperation::VideoCompression).unwrap(),
            "\"videoCompression\""
        );
        assert_eq!(
            serde_json::from_str::<ReindexOperation>("\"fileSize\"").unwrap(),
            ReindexOperation::FileSize
        );
    }

    #[test]
    fn plan_rejects_empty_and_uses_canonical_operation_order() {
        assert!(MediaTaskPlan::new(Vec::new()).is_err());
        let plan = MediaTaskPlan::new(vec![
            ReindexOperation::ClearTags,
            ReindexOperation::Exif,
            ReindexOperation::Exif,
            ReindexOperation::FileSize,
        ])
        .unwrap();
        assert_eq!(
            plan.operations(),
            &[
                ReindexOperation::Exif,
                ReindexOperation::FileSize,
                ReindexOperation::ClearTags
            ]
        );
    }

    #[test]
    fn safe_plan_stage_order_is_fixed() {
        let stages = MediaTaskPlan::safe_default().stages_for(&image());
        assert_eq!(
            stages,
            vec![
                MediaStage::FileSize,
                MediaStage::MetadataProbe,
                MediaStage::DecodeAndDimensions,
                MediaStage::Thumbnail,
                MediaStage::VisualHashes,
                MediaStage::Publish,
            ]
        );
    }

    #[test]
    fn operations_have_expected_image_and_video_applicability() {
        for operation in ReindexOperation::ALL {
            assert!(operation.applies_to(false));
            assert_eq!(
                operation.applies_to(true),
                operation != ReindexOperation::VideoCompression
            );
        }
    }

    #[test]
    fn compression_only_plan_does_not_schedule_thumbnail_or_hashes() {
        let plan = MediaTaskPlan::new(vec![ReindexOperation::VideoCompression]).unwrap();
        assert_eq!(
            plan.stages_for(&video()),
            vec![
                MediaStage::MetadataProbe,
                MediaStage::VideoCompression,
                MediaStage::Publish
            ]
        );
        assert!(!plan.has_applicable_operation(&image()));
    }

    #[test]
    fn thumbnail_and_visual_hashes_remain_independent_stages() {
        let thumbnail = MediaTaskPlan::new(vec![ReindexOperation::Thumbnail]).unwrap();
        assert_eq!(
            thumbnail.stages_for(&image()),
            vec![
                MediaStage::MetadataProbe,
                MediaStage::DecodeAndDimensions,
                MediaStage::Thumbnail,
                MediaStage::Publish,
            ]
        );

        let hashes = MediaTaskPlan::new(vec![ReindexOperation::VisualHashes]).unwrap();
        assert_eq!(
            hashes.stages_for(&image()),
            vec![
                MediaStage::MetadataProbe,
                MediaStage::DecodeAndDimensions,
                MediaStage::VisualHashes,
                MediaStage::Publish,
            ]
        );
    }

    #[test]
    fn applying_hash_only_preserves_unselected_fields_and_user_metadata() {
        let mut latest = image();
        latest.tag_mut().insert("keep".to_string());
        if let AbstractData::Image(image) = &mut latest {
            image.object.description = Some("keep description".to_string());
            image.metadata.exif_vec = BTreeMap::from([("Make".to_string(), "old".to_string())]);
        }
        let mut candidate = latest.clone();
        candidate.set_size(999);
        candidate.set_width(888);
        candidate.set_height(777);
        candidate.set_thumbhash(vec![1, 2, 3]);
        candidate.set_phash(vec![4, 5, 6]);
        if let Some(exif) = candidate.exif_vec_mut() {
            *exif = BTreeMap::from([("Make".to_string(), "new".to_string())]);
        }
        let plan = MediaTaskPlan::new(vec![ReindexOperation::VisualHashes]).unwrap();
        apply_selected_outputs(
            &mut latest,
            &MediaPipelineResult {
                candidate,
                static_gif_conversion: false,
            },
            &plan,
        )
        .unwrap();
        let AbstractData::Image(image) = latest else {
            panic!("expected image");
        };
        assert_eq!(image.metadata.size, 10);
        assert_eq!((image.metadata.width, image.metadata.height), (20, 30));
        assert_eq!(image.metadata.exif_vec["Make"], "old");
        assert_eq!(image.object.thumbhash, Some(vec![1, 2, 3]));
        assert_eq!(image.metadata.phash, Some(vec![4, 5, 6]));
        assert!(image.object.tags.contains("keep"));
        assert_eq!(
            image.object.description.as_deref(),
            Some("keep description")
        );
    }

    #[test]
    fn clear_tags_is_applied_at_patch_time_without_touching_other_fields() {
        let mut latest = image();
        latest
            .tag_mut()
            .extend(["one".to_string(), "two".to_string()]);
        let candidate = latest.clone();
        let plan = MediaTaskPlan::new(vec![ReindexOperation::ClearTags]).unwrap();
        apply_selected_outputs(
            &mut latest,
            &MediaPipelineResult {
                candidate,
                static_gif_conversion: false,
            },
            &plan,
        )
        .unwrap();
        assert!(latest.tag().is_empty());
        assert_eq!(latest.width(), 20);
    }

    #[test]
    fn static_gif_conversion_forces_full_image_outputs_but_preserves_tags() {
        let mut latest = video();
        latest.tag_mut().insert("keep".to_string());
        let mut candidate = latest.clone();
        candidate.convert_to_image();
        candidate.set_size(91);
        candidate.set_width(92);
        candidate.set_height(93);
        candidate.set_thumbhash(vec![9]);
        candidate.set_phash(vec![8]);
        let plan = MediaTaskPlan::new(vec![ReindexOperation::VideoCompression]).unwrap();
        apply_selected_outputs(
            &mut latest,
            &MediaPipelineResult {
                candidate,
                static_gif_conversion: true,
            },
            &plan,
        )
        .unwrap();
        let AbstractData::Image(image) = latest else {
            panic!("expected static GIF to become an image");
        };
        assert_eq!(image.metadata.size, 91);
        assert_eq!((image.metadata.width, image.metadata.height), (92, 93));
        assert_eq!(image.object.thumbhash, Some(vec![9]));
        assert!(image.object.tags.contains("keep"));
    }
}
