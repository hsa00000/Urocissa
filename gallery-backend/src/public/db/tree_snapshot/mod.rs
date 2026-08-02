pub mod new;
pub mod read_rows;
pub mod read_scrollbar;
pub mod read_tree_snapshot;

use std::sync::LazyLock;

use dashmap::DashMap;
use redb::TableDefinition;

use crate::public::db::tree::state::{SlotRef, TargetSet};
use crate::public::structure::response::row::ScrollBarData;

pub const TREE_SNAPSHOT_TABLE: TableDefinition<i64, &[u8]> = TableDefinition::new("tree_snapshots");
pub const SCROLLBAR_METADATA_TABLE: TableDefinition<i64, &[u8]> =
    TableDefinition::new("scrollbar_metadata");

const SNAPSHOT_MAGIC: [u8; 4] = *b"UTS6";
const SNAPSHOT_HEADER_BYTES: usize = 28;
const SNAPSHOT_CHECKSUM_BYTES: usize = 8;

#[derive(Debug, Clone)]
pub struct PendingTreeSnapshot {
    pub structural_epoch: u64,
    pub universe: usize,
    pub ordinals: Vec<u32>,
    pub targets: TargetSet,
    pub scrollbar: Vec<ScrollBarData>,
}

impl PendingTreeSnapshot {
    #[cfg(feature = "performance-test")]
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(
                self.ordinals
                    .capacity()
                    .saturating_mul(std::mem::size_of::<u32>()),
            )
            .saturating_add(self.targets.estimated_bytes())
            .saturating_add(
                self.scrollbar
                    .capacity()
                    .saturating_mul(std::mem::size_of::<ScrollBarData>()),
            )
    }

    #[cfg(test)]
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        self.encode_with_layout().map(|(bytes, _)| bytes)
    }

    pub fn encode_with_layout(&self) -> anyhow::Result<(Vec<u8>, SnapshotBlobLayout)> {
        let ordinal_count = u32::try_from(self.ordinals.len())?;
        let (words, overrides) = self.targets.dense_parts(self.universe);
        let word_count = u32::try_from(words.len())?;
        let override_count = u32::try_from(overrides.len())?;
        let payload_bytes = self
            .ordinals
            .len()
            .checked_mul(4)
            .and_then(|value| value.checked_add(words.len().checked_mul(8)?))
            .and_then(|value| value.checked_add(overrides.len().checked_mul(8)?))
            .ok_or_else(|| anyhow::anyhow!("tree snapshot is too large"))?;
        let mut bytes =
            Vec::with_capacity(SNAPSHOT_HEADER_BYTES + payload_bytes + SNAPSHOT_CHECKSUM_BYTES);
        bytes.extend_from_slice(&SNAPSHOT_MAGIC);
        bytes.extend_from_slice(&self.structural_epoch.to_le_bytes());
        bytes.extend_from_slice(&u32::try_from(self.universe)?.to_le_bytes());
        bytes.extend_from_slice(&ordinal_count.to_le_bytes());
        bytes.extend_from_slice(&word_count.to_le_bytes());
        bytes.extend_from_slice(&override_count.to_le_bytes());
        for ordinal in &self.ordinals {
            bytes.extend_from_slice(&ordinal.to_le_bytes());
        }
        for word in words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for (ordinal, generation) in overrides {
            bytes.extend_from_slice(&ordinal.to_le_bytes());
            bytes.extend_from_slice(&generation.to_le_bytes());
        }
        let checksum = blake3::hash(&bytes);
        bytes.extend_from_slice(&checksum.as_bytes()[..SNAPSHOT_CHECKSUM_BYTES]);
        Ok((
            bytes,
            SnapshotBlobLayout {
                structural_epoch: self.structural_epoch,
                universe: self.universe,
                ordinal_count: ordinal_count as usize,
                word_count: word_count as usize,
                override_count: override_count as usize,
            },
        ))
    }
}

pub struct SnapshotBlobView<'a> {
    bytes: &'a [u8],
    structural_epoch: u64,
    universe: usize,
    ordinal_count: usize,
    word_count: usize,
    override_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapshotBlobLayout {
    structural_epoch: u64,
    universe: usize,
    ordinal_count: usize,
    word_count: usize,
    override_count: usize,
}

impl<'a> SnapshotBlobView<'a> {
    pub fn new(bytes: &'a [u8]) -> anyhow::Result<Self> {
        if bytes.get(..4) != Some(SNAPSHOT_MAGIC.as_slice()) {
            return Err(anyhow::anyhow!("tree snapshot magic is invalid"));
        }
        let structural_epoch = read_u64(bytes, 4)?;
        let universe = read_u32(bytes, 12)? as usize;
        let ordinal_count = read_u32(bytes, 16)? as usize;
        let word_count = read_u32(bytes, 20)? as usize;
        let override_count = read_u32(bytes, 24)? as usize;
        let expected = SNAPSHOT_HEADER_BYTES
            .checked_add(
                ordinal_count
                    .checked_mul(4)
                    .ok_or_else(|| anyhow::anyhow!("tree snapshot ordinal length overflow"))?,
            )
            .and_then(|value| value.checked_add(word_count.checked_mul(8)?))
            .and_then(|value| value.checked_add(override_count.checked_mul(8)?))
            .and_then(|value| value.checked_add(SNAPSHOT_CHECKSUM_BYTES))
            .ok_or_else(|| anyhow::anyhow!("tree snapshot length overflow"))?;
        if bytes.len() != expected {
            return Err(anyhow::anyhow!(
                "tree snapshot length mismatch: expected {expected}, got {}",
                bytes.len()
            ));
        }
        let checksum_offset = bytes.len() - SNAPSHOT_CHECKSUM_BYTES;
        let expected_checksum = blake3::hash(&bytes[..checksum_offset]);
        if bytes[checksum_offset..] != expected_checksum.as_bytes()[..SNAPSHOT_CHECKSUM_BYTES] {
            return Err(anyhow::anyhow!("tree snapshot checksum is invalid"));
        }
        if word_count != universe.saturating_add(63) / 64 {
            return Err(anyhow::anyhow!("tree snapshot bitmap length is invalid"));
        }
        Ok(Self {
            bytes,
            structural_epoch,
            universe,
            ordinal_count,
            word_count,
            override_count,
        })
    }

    pub fn layout(&self) -> SnapshotBlobLayout {
        SnapshotBlobLayout {
            structural_epoch: self.structural_epoch,
            universe: self.universe,
            ordinal_count: self.ordinal_count,
            word_count: self.word_count,
            override_count: self.override_count,
        }
    }

    pub fn from_verified_layout(bytes: &'a [u8], layout: SnapshotBlobLayout) -> Self {
        Self {
            bytes,
            structural_epoch: layout.structural_epoch,
            universe: layout.universe,
            ordinal_count: layout.ordinal_count,
            word_count: layout.word_count,
            override_count: layout.override_count,
        }
    }

    pub fn structural_epoch(&self) -> u64 {
        self.structural_epoch
    }

    pub fn len(&self) -> usize {
        self.ordinal_count
    }

    pub fn ordinal(&self, index: usize) -> anyhow::Result<u32> {
        if index >= self.ordinal_count {
            return Err(anyhow::anyhow!(
                "tree snapshot index {index} is out of range"
            ));
        }
        read_u32(self.bytes, SNAPSHOT_HEADER_BYTES + index * 4)
    }

    pub fn slot_ref(&self, index: usize) -> anyhow::Result<SlotRef> {
        let ordinal = self.ordinal(index)?;
        let generation = self.generation_for(ordinal)?;
        Ok(SlotRef::new(ordinal, generation))
    }

    pub fn for_each_slot_ref(
        &self,
        mut visit: impl FnMut(usize, SlotRef) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        for index in 0..self.ordinal_count {
            visit(index, self.slot_ref(index)?)?;
        }
        Ok(())
    }

    pub fn target_set(&self) -> anyhow::Result<TargetSet> {
        let words_offset = SNAPSHOT_HEADER_BYTES + self.ordinal_count * 4;
        let words = (0..self.word_count)
            .map(|index| read_u64(self.bytes, words_offset + index * 8))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let overrides_offset = words_offset + self.word_count * 8;
        let overrides = (0..self.override_count)
            .map(|index| {
                let offset = overrides_offset + index * 8;
                Ok((
                    read_u32(self.bytes, offset)?,
                    read_u32(self.bytes, offset + 4)?,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let targets = TargetSet::from_dense_parts(words, overrides);
        if targets.len() != self.ordinal_count {
            return Err(anyhow::anyhow!(
                "tree snapshot target cardinality does not match its order"
            ));
        }
        Ok(targets)
    }

    fn generation_for(&self, ordinal: u32) -> anyhow::Result<u32> {
        let overrides_offset = SNAPSHOT_HEADER_BYTES + self.ordinal_count * 4 + self.word_count * 8;
        let mut left = 0;
        let mut right = self.override_count;
        while left < right {
            let middle = left + (right - left) / 2;
            let offset = overrides_offset + middle * 8;
            match read_u32(self.bytes, offset)?.cmp(&ordinal) {
                std::cmp::Ordering::Less => left = middle + 1,
                std::cmp::Ordering::Greater => right = middle,
                std::cmp::Ordering::Equal => return read_u32(self.bytes, offset + 4),
            }
        }
        Ok(1)
    }

    pub fn universe(&self) -> usize {
        self.universe
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> anyhow::Result<u32> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow::anyhow!("tree snapshot u32 is truncated"))?;
    Ok(u32::from_le_bytes(value.try_into()?))
}

fn read_u64(bytes: &[u8], offset: usize) -> anyhow::Result<u64> {
    let value = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| anyhow::anyhow!("tree snapshot u64 is truncated"))?;
    Ok(u64::from_le_bytes(value.try_into()?))
}

#[derive(Debug)]
pub struct TreeSnapshot {
    pub in_disk: &'static redb::Database,
    /// Ordered compact arena ordinals plus a structural epoch. Static fields
    /// are resolved from `RecordArena`, avoiding full metadata copies.
    pub in_memory: &'static DashMap<i64, PendingTreeSnapshot>,
    pub verified_layouts: &'static DashMap<i64, SnapshotBlobLayout>,
}

pub static TREE_SNAPSHOT: LazyLock<TreeSnapshot> = LazyLock::new(TreeSnapshot::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_snapshot_round_trips_order_bitmap_epoch_and_generations() {
        let slots = [SlotRef::new(65, 1), SlotRef::new(7, 2), SlotRef::new(1, 1)];
        let snapshot = PendingTreeSnapshot {
            structural_epoch: 42,
            universe: 128,
            ordinals: slots.iter().map(|slot| slot.index()).collect(),
            targets: TargetSet::from_unique_slot_refs(slots, 128),
            scrollbar: Vec::new(),
        };
        let bytes = snapshot.encode().unwrap();
        #[cfg(feature = "performance-test")]
        assert!(snapshot.estimated_bytes() >= std::mem::size_of::<PendingTreeSnapshot>());
        assert!(bytes.len() < 256);
        let view = SnapshotBlobView::new(&bytes).unwrap();
        assert_eq!(view.structural_epoch(), 42);
        assert_eq!(view.universe(), 128);
        assert_eq!(view.len(), 3);
        assert_eq!(view.slot_ref(0).unwrap(), SlotRef::new(65, 1));
        assert_eq!(view.slot_ref(1).unwrap(), SlotRef::new(7, 2));
        assert_eq!(view.slot_ref(2).unwrap(), SlotRef::new(1, 1));
        assert_eq!(
            view.target_set().unwrap().iter().collect::<Vec<_>>(),
            snapshot.targets.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn compact_snapshot_rejects_corruption_and_cardinality_mismatch() {
        let snapshot = PendingTreeSnapshot {
            structural_epoch: 7,
            universe: 64,
            ordinals: vec![1],
            targets: TargetSet::from_slot_refs([SlotRef::new(1, 1)], 64),
            scrollbar: Vec::new(),
        };
        let bytes = snapshot.encode().unwrap();
        assert!(SnapshotBlobView::new(&bytes[..bytes.len() - 1]).is_err());

        let mut bad_magic = bytes.clone();
        bad_magic[0] = 0;
        assert!(SnapshotBlobView::new(&bad_magic).is_err());

        let mut bad_bitmap = bytes;
        let bitmap_offset = SNAPSHOT_HEADER_BYTES + 4;
        bad_bitmap[bitmap_offset..bitmap_offset + 8].fill(0);
        assert!(SnapshotBlobView::new(&bad_bitmap).is_err());
    }
}
