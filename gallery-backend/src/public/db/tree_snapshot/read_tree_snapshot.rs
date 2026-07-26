use super::{
    PendingTreeSnapshot, SnapshotBlobLayout, SnapshotBlobView, TREE_SNAPSHOT_TABLE, TreeSnapshot,
};
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::{SlotRef, TargetSet};
use anyhow::{Context, Result};
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;
use redb::{ReadOnlyTable, ReadableDatabase};

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: i64) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(&timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        let read_txn = self.in_disk.begin_read()?;
        let table = read_txn.open_table(TREE_SNAPSHOT_TABLE)?;
        let layout = if let Some(layout) = self.verified_layouts.get(&timestamp) {
            *layout.value()
        } else {
            let layout = {
                let value = table
                    .get(timestamp)?
                    .context(format!("tree snapshot {timestamp} does not exist"))?;
                SnapshotBlobView::new(value.value())?.layout()
            };
            self.verified_layouts.insert(timestamp, layout);
            layout
        };
        Ok(MyCow::Redb {
            table,
            timestamp,
            layout,
        })
    }
}

pub(crate) enum PinnedSnapshotView<'a> {
    Memory(&'a PendingTreeSnapshot),
    Redb(SnapshotBlobView<'a>),
}

impl PinnedSnapshotView<'_> {
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Memory(snapshot) => snapshot.ordinals.len(),
            Self::Redb(snapshot) => snapshot.len(),
        }
    }

    pub(crate) fn structural_epoch(&self) -> u64 {
        match self {
            Self::Memory(snapshot) => snapshot.structural_epoch,
            Self::Redb(snapshot) => snapshot.structural_epoch(),
        }
    }

    pub(crate) fn universe(&self) -> usize {
        match self {
            Self::Memory(snapshot) => snapshot.universe,
            Self::Redb(snapshot) => snapshot.universe(),
        }
    }

    pub(crate) fn target_set(&self) -> Result<TargetSet> {
        match self {
            Self::Memory(snapshot) => Ok(snapshot.targets.clone()),
            Self::Redb(snapshot) => snapshot.target_set(),
        }
    }

    pub(crate) fn slot_ref(&self, index: usize) -> Result<SlotRef> {
        match self {
            Self::Memory(snapshot) => {
                let ordinal = *snapshot
                    .ordinals
                    .get(index)
                    .context(format!("Failed to find slot reference at index {index}"))?;
                snapshot
                    .targets
                    .slot_ref_for_ordinal(ordinal)
                    .context("tree snapshot target bitmap is inconsistent")
            }
            Self::Redb(snapshot) => snapshot.slot_ref(index),
        }
    }

    pub(crate) fn for_each_selected_slot_ref(
        &self,
        indices: &[u32],
        mut visit: impl FnMut(SlotRef),
    ) -> Result<()> {
        for index in indices {
            visit(self.slot_ref(*index as usize)?);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum MyCow {
    DashMap(Ref<'static, i64, PendingTreeSnapshot>),
    Redb {
        table: ReadOnlyTable<i64, &'static [u8]>,
        timestamp: i64,
        layout: SnapshotBlobLayout,
    },
}

impl MyCow {
    pub(crate) fn with_pinned_view<R>(
        &self,
        operation: impl FnOnce(PinnedSnapshotView<'_>) -> R,
    ) -> Result<R> {
        match self {
            Self::DashMap(data) => Ok(operation(PinnedSnapshotView::Memory(data.value()))),
            Self::Redb {
                table,
                timestamp,
                layout,
            } => {
                let value = table
                    .get(*timestamp)?
                    .context(format!("tree snapshot {timestamp} disappeared"))?;
                // The read-only Redb transaction pins immutable bytes. They were
                // checksum-verified once when this snapshot handle was opened.
                let view = SnapshotBlobView::from_verified_layout(value.value(), *layout);
                Ok(operation(PinnedSnapshotView::Redb(view)))
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::DashMap(data) => data.value().ordinals.len(),
            Self::Redb { .. } => self.with_disk_view(|view| view.len()).unwrap_or(0),
        }
    }

    pub fn structural_epoch(&self) -> Result<u64> {
        match self {
            Self::DashMap(data) => Ok(data.value().structural_epoch),
            Self::Redb { .. } => self.with_disk_view(|view| view.structural_epoch()),
        }
    }

    pub fn universe(&self) -> Result<usize> {
        match self {
            Self::DashMap(data) => Ok(data.value().universe),
            Self::Redb { .. } => self.with_disk_view(|view| view.universe()),
        }
    }

    pub fn target_set(&self) -> Result<TargetSet> {
        match self {
            Self::DashMap(data) => Ok(data.value().targets.clone()),
            Self::Redb { .. } => self.with_disk_view(|view| view.target_set())?,
        }
    }

    pub fn get_width_height(&self, index: usize) -> Result<(u32, u32)> {
        let slot_ref = SlotRef::from_raw(self.get_slot_ref(index)?);
        let state = TREE
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("tree state lock poisoned"))?;
        let record = state
            .get(slot_ref)
            .context(format!("stale tree snapshot generation at index {index}"))?;
        Ok((record.width, record.height))
    }

    pub fn get_hash(&self, index: usize) -> Result<ArrayString<64>> {
        let slot_ref = SlotRef::from_raw(self.get_slot_ref(index)?);
        let state = TREE
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("tree state lock poisoned"))?;
        state
            .get(slot_ref)
            .map(|record| record.id)
            .context(format!("stale tree snapshot generation at index {index}"))
    }

    pub fn get_identity(&self, index: usize) -> Result<(ArrayString<64>, u64)> {
        let raw = self.get_slot_ref(index)?;
        let slot_ref = SlotRef::from_raw(raw);
        let state = TREE
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("tree state lock poisoned"))?;
        let hash = state
            .get(slot_ref)
            .map(|record| record.id)
            .context(format!("stale tree snapshot generation at index {index}"))?;
        Ok((hash, raw))
    }

    pub fn for_each_slot_ref(&self, mut visit: impl FnMut(usize, u64) -> Result<()>) -> Result<()> {
        match self {
            Self::DashMap(data) => {
                for (index, ordinal) in data.value().ordinals.iter().copied().enumerate() {
                    let slot_ref = data
                        .value()
                        .targets
                        .slot_ref_for_ordinal(ordinal)
                        .context("tree snapshot target bitmap is inconsistent")?;
                    visit(index, slot_ref.raw())?;
                }
            }
            Self::Redb { .. } => self.with_disk_view(|view| {
                view.for_each_slot_ref(|index, slot_ref| visit(index, slot_ref.raw()))
            })??,
        }
        Ok(())
    }

    pub fn for_each_timestamp(
        &self,
        mut visit: impl FnMut(usize, i64) -> Result<()>,
    ) -> Result<()> {
        let state = TREE
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("tree state lock poisoned"))?;
        self.for_each_slot_ref(|index, raw| {
            let record = state
                .get(SlotRef::from_raw(raw))
                .context(format!("stale tree snapshot generation at index {index}"))?;
            visit(index, record.timestamp)
        })
    }

    pub fn get_slot_ref(&self, index: usize) -> Result<u64> {
        match self {
            Self::DashMap(data) => {
                let ordinal = *data
                    .value()
                    .ordinals
                    .get(index)
                    .context(format!("Failed to find slot reference at index {index}"))?;
                data.value()
                    .targets
                    .slot_ref_for_ordinal(ordinal)
                    .map(SlotRef::raw)
                    .context("tree snapshot target bitmap is inconsistent")
            }
            Self::Redb { .. } => self
                .with_disk_view(|view| view.slot_ref(index))?
                .map(SlotRef::raw),
        }
    }

    fn with_disk_view<R>(&self, operation: impl FnOnce(&SnapshotBlobView<'_>) -> R) -> Result<R> {
        let Self::Redb {
            table,
            timestamp,
            layout,
        } = self
        else {
            return Err(anyhow::anyhow!("snapshot is not disk-backed"));
        };
        let value = table
            .get(*timestamp)?
            .context(format!("tree snapshot {timestamp} disappeared"))?;
        // The read-only Redb transaction pins immutable bytes. They were
        // checksum-verified once when this snapshot handle was opened.
        let view = SnapshotBlobView::from_verified_layout(value.value(), *layout);
        Ok(operation(&view))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redb_snapshot_uses_one_pinned_view_for_bulk_resolution() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let database = redb::Database::create(directory.path().join("snapshot.redb"))?;
        let slots = [SlotRef::new(5, 1), SlotRef::new(1, 4), SlotRef::new(9, 1)];
        let expected_targets = TargetSet::from_unique_slot_refs(slots, 16);
        let snapshot = PendingTreeSnapshot {
            structural_epoch: 77,
            universe: 16,
            ordinals: slots.iter().map(|slot_ref| slot_ref.index()).collect(),
            targets: expected_targets.clone(),
            scrollbar: Vec::new(),
        };
        let (bytes, layout) = snapshot.encode_with_layout()?;
        let write = database.begin_write()?;
        {
            let mut table = write.open_table(TREE_SNAPSHOT_TABLE)?;
            table.insert(123, bytes.as_slice())?;
        }
        write.commit()?;

        let read = database.begin_read()?;
        let table = read.open_table(TREE_SNAPSHOT_TABLE)?;
        let snapshot = MyCow::Redb {
            table,
            timestamp: 123,
            layout,
        };
        let resolved = snapshot.with_pinned_view(|view| -> Result<_> {
            assert_eq!(view.len(), 3);
            assert_eq!(view.structural_epoch(), 77);
            assert_eq!(view.universe(), 16);
            let mut resolved = Vec::new();
            view.for_each_selected_slot_ref(&[2, 0, 1], |slot_ref| {
                resolved.push(slot_ref);
            })?;
            Ok((resolved, view.target_set()?))
        })??;

        assert_eq!(resolved.0, vec![slots[2], slots[0], slots[1]]);
        assert_eq!(resolved.1, expected_targets);
        Ok(())
    }
}
