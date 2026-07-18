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
