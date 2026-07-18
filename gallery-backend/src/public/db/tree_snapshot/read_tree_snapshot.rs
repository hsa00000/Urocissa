use super::{PendingTreeSnapshot, TreeSnapshot};
use crate::public::db::tree::TREE;
use crate::public::db::tree::state::SlotRef;
use anyhow::Context;
use anyhow::Result;
use arrayvec::ArrayString;
use dashmap::mapref::one::Ref;
use redb::{
    ReadOnlyTable, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition,
};

impl TreeSnapshot {
    pub fn read_tree_snapshot(&'static self, timestamp: i64) -> Result<MyCow> {
        if let Some(data) = self.in_memory.get(&timestamp) {
            return Ok(MyCow::DashMap(data));
        }

        let read_txn = self.in_disk.begin_read()?;

        let binding = timestamp.to_string();
        let table_definition: TableDefinition<u64, u64> = TableDefinition::new(&binding);

        let table = read_txn.open_table(table_definition)?;
        Ok(MyCow::Redb(table))
    }
}

#[derive(Debug)]
pub enum MyCow {
    DashMap(Ref<'static, i64, PendingTreeSnapshot>),
    Redb(ReadOnlyTable<u64, u64>),
}

impl MyCow {
    #[allow(clippy::cast_possible_truncation)]
    pub fn len(&self) -> usize {
        match self {
            MyCow::DashMap(data) => data.value().slots.len(),
            MyCow::Redb(table) => table.len().unwrap() as usize,
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
            MyCow::DashMap(data) => {
                for (index, slot_ref) in data.value().slots.iter().copied().enumerate() {
                    visit(index, slot_ref)?;
                }
            }
            MyCow::Redb(table) => {
                for entry in table.iter()? {
                    let (index, value) = entry?;
                    visit(index.value() as usize, value.value())?;
                }
            }
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
            MyCow::DashMap(data) => data
                .value()
                .slots
                .get(index)
                .copied()
                .context(format!("Failed to find slot reference at index {index}")),
            MyCow::Redb(table) => {
                let guard = table.get(index as u64)?.context(format!(
                    "Failed to find slot reference in tree snapshot for index {index}"
                ))?;
                Ok(guard.value())
            }
        }
    }
}
