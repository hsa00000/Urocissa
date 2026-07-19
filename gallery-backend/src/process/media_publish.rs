use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::Ordering;

use anyhow::{Context, Result, anyhow};
use arrayvec::ArrayString;

use crate::process::artifact_publisher::ArtifactPublisher;
use crate::process::media_pipeline::{MediaPipelineResult, MediaTaskPlan, apply_selected_outputs};
use crate::public::db::tree::state::{SlotRef, TargetSet};
use crate::public::db::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::public::db::write_behind::WRITE_BEHIND;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::object::next_mutation_timestamp;

pub fn load_logical_media(object_id: ArrayString<64>) -> Result<(SlotRef, AbstractData)> {
    let slot_ref = TREE
        .state
        .read()
        .map_err(|_| anyhow!("tree state lock poisoned"))?
        .find(object_id.as_str())
        .ok_or_else(|| anyhow!("media object not found"))?;
    let durable = TREE.store.read(|reader| {
        Ok::<_, anyhow::Error>(
            reader
                .get(object_id.as_str())?
                .map(crate::storage::store::RecordValue::into_value),
        )
    })?;
    let logical = WRITE_BEHIND
        .logical_record_for_slot(Some(slot_ref), object_id.as_str(), durable)
        .ok_or_else(|| anyhow!("media object was deleted"))?;
    Ok((slot_ref, logical))
}

/// Atomically (within one running process) publish staged media artifacts and
/// a selective metadata patch for a stable arena slot.
pub fn publish_reindex_result(
    slot_ref: SlotRef,
    object_id: ArrayString<64>,
    plan: &MediaTaskPlan,
    result: &MediaPipelineResult,
    publisher: ArtifactPublisher,
) -> Result<AbstractData> {
    publish_media_mutation(slot_ref, object_id, publisher, |latest| {
        if plan.contains(crate::process::media_pipeline::ReindexOperation::Thumbnail)
            || result.static_gif_conversion
        {
            let expected_previous = result
                .candidate
                .cache_version()
                .checked_sub(1)
                .ok_or_else(|| anyhow!("replacement thumbnail did not advance cache version"))?;
            if latest.cache_version() != expected_previous {
                anyhow::bail!("thumbnail cache version changed before reindex was published");
            }
        }
        apply_selected_outputs(latest, result, plan)
            .context("failed to apply selective media patch")
    })
}

/// Shared commit path for every operation that mutates media artifacts. The
/// closure receives the newest durable record with its write-behind overlay.
pub fn publish_media_mutation(
    slot_ref: SlotRef,
    object_id: ArrayString<64>,
    publisher: ArtifactPublisher,
    mutation: impl FnOnce(&mut AbstractData) -> Result<()>,
) -> Result<AbstractData> {
    publisher.publish(|| {
        let _persistence_guard = TREE
            .persistence_lock
            .lock()
            .map_err(|_| anyhow!("tree persistence lock poisoned"))?;
        let mut state = TREE
            .state
            .write()
            .map_err(|_| anyhow!("tree state lock poisoned"))?;
        let current = state
            .get(slot_ref)
            .filter(|record| record.id == object_id)
            .ok_or_else(|| anyhow!("object slot no longer exists at its captured generation"))?;
        debug_assert_eq!(current.id, object_id);
        let changed_at = next_mutation_timestamp();

        let durable = TREE.store.read(|reader| {
            Ok::<_, anyhow::Error>(
                reader
                    .get(object_id.as_str())?
                    .map(crate::storage::store::RecordValue::into_value),
            )
        })?;
        let mut album_ids = durable
            .as_ref()
            .and_then(AbstractData::albums)
            .into_iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut latest = WRITE_BEHIND
            .logical_record_for_slot(Some(slot_ref), object_id.as_str(), durable)
            .ok_or_else(|| anyhow!("object was deleted before publication"))?;
        mutation(&mut latest)?;
        latest.touch_update_at(changed_at);

        album_ids.extend(latest.albums().into_iter().flatten().copied());
        let (published_album_ids, albums): (BTreeSet<_>, Vec<_>) = album_ids
            .iter()
            .filter_map(|album_id| {
                state
                    .album_aggregate_with_override(*album_id, slot_ref, &latest, changed_at)
                    .map(|album| (*album_id, AbstractData::Album(album)))
            })
            .unzip();

        TREE.store.write(|writer| {
            writer.insert(&latest)?;
            for album in &albums {
                writer.insert(album)?;
            }
            Ok::<(), anyhow::Error>(())
        })?;

        let target = TargetSet::from_slot_refs([slot_ref], state.arena.capacity());
        WRITE_BEHIND.cancel_published_media(&target, &published_album_ids);
        let mut records = Vec::with_capacity(albums.len() + 1);
        records.push(latest.clone());
        records.extend(albums);
        state.apply_batch(&records, &HashSet::new());
        VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
        Ok(latest)
    })
}
