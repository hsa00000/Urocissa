use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, LazyLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use arrayvec::ArrayString;

use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::AlbumCombined;
use crate::public::structure::expression::{AlbumFilterValue, Expression, FilterValue};
use crate::public::structure::object::{ObjectSchema, ObjectType};
use crate::public::structure::response::reduced_data::ReducedData;

static NEXT_STRUCTURAL_EPOCH: LazyLock<AtomicU64> = LazyLock::new(|| {
    let time_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |duration| {
            u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
        });
    AtomicU64::new((time_seed ^ u64::from(std::process::id())).max(1))
});

fn next_structural_epoch() -> u64 {
    NEXT_STRUCTURAL_EPOCH.fetch_add(1, AtomicOrdering::Relaxed)
}

/// A stable arena reference. Reusing a slot always changes its generation, so a
/// reference captured by an old selection can never silently point at a new
/// object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotRef(u64);

impl SlotRef {
    pub const fn new(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << 32) | index as u64)
    }

    pub const fn index(self) -> u32 {
        self.0 as u32
    }

    pub const fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
struct ArenaSlot<T> {
    generation: u32,
    value: Option<T>,
}

#[derive(Debug)]
pub struct RecordArena<T> {
    slots: Vec<ArenaSlot<T>>,
    free: Vec<u32>,
    len: usize,
}

impl<T> Default for RecordArena<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }
}

impl<T> RecordArena<T> {
    pub fn allocate(&mut self, value: T) -> SlotRef {
        self.len += 1;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.generation = slot.generation.wrapping_add(1).max(1);
            slot.value = Some(value);
            return SlotRef::new(index, slot.generation);
        }

        let index = u32::try_from(self.slots.len()).expect("record arena exceeded u32 capacity");
        self.slots.push(ArenaSlot {
            generation: 1,
            value: Some(value),
        });
        SlotRef::new(index, 1)
    }

    pub fn get(&self, slot_ref: SlotRef) -> Option<&T> {
        let slot = self.slots.get(slot_ref.index() as usize)?;
        (slot.generation == slot_ref.generation())
            .then_some(slot.value.as_ref())
            .flatten()
    }

    pub fn get_mut(&mut self, slot_ref: SlotRef) -> Option<&mut T> {
        let slot = self.slots.get_mut(slot_ref.index() as usize)?;
        (slot.generation == slot_ref.generation())
            .then_some(slot.value.as_mut())
            .flatten()
    }

    pub fn remove(&mut self, slot_ref: SlotRef) -> Option<T> {
        let slot = self.slots.get_mut(slot_ref.index() as usize)?;
        if slot.generation != slot_ref.generation() {
            return None;
        }
        let value = slot.value.take()?;
        self.len -= 1;
        self.free.push(slot_ref.index());
        Some(value)
    }

    pub fn slot_at_ordinal(&self, ordinal: u32) -> Option<SlotRef> {
        let slot = self.slots.get(ordinal as usize)?;
        slot.value
            .as_ref()
            .map(|_| SlotRef::new(ordinal, slot.generation))
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    fn future_capacity(&self, additions: usize, removals: usize) -> usize {
        self.slots.len() + additions.saturating_sub(self.free.len().saturating_add(removals))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Debug, Default)]
pub struct IdIndex {
    primary: HashMap<u64, SlotRef>,
    collisions: HashMap<u64, Vec<SlotRef>>,
}

impl IdIndex {
    fn fingerprint(id: &str) -> u64 {
        let bytes = blake3::hash(id.as_bytes());
        u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("eight-byte slice"))
    }

    pub fn insert(&mut self, id: &str, slot_ref: SlotRef) {
        use std::collections::hash_map::Entry;
        let fingerprint = Self::fingerprint(id);
        if let Some(bucket) = self.collisions.get_mut(&fingerprint) {
            if !bucket.contains(&slot_ref) {
                bucket.push(slot_ref);
            }
            return;
        }
        match self.primary.entry(fingerprint) {
            Entry::Vacant(entry) => {
                entry.insert(slot_ref);
            }
            Entry::Occupied(entry) => {
                let existing = *entry.get();
                if existing != slot_ref {
                    self.collisions
                        .insert(fingerprint, vec![existing, slot_ref]);
                }
            }
        }
    }

    pub fn find(&self, id: &str, arena: &RecordArena<CacheRecord>) -> Option<SlotRef> {
        let matches = |slot_ref: &SlotRef| {
            arena
                .get(*slot_ref)
                .is_some_and(|record| record.id.as_str() == id)
        };
        let fingerprint = Self::fingerprint(id);
        if let Some(bucket) = self.collisions.get(&fingerprint) {
            return bucket.iter().find(|slot_ref| matches(slot_ref)).copied();
        }
        let slot_ref = self.primary.get(&fingerprint)?;
        matches(slot_ref).then_some(*slot_ref)
    }

    pub fn remove(&mut self, id: &str, slot_ref: SlotRef) {
        let fingerprint = Self::fingerprint(id);
        if let Some(bucket) = self.collisions.get_mut(&fingerprint) {
            bucket.retain(|item| *item != slot_ref);
            let remaining = bucket.first().copied();
            if bucket.len() > 1 {
                if let Some(remaining) = remaining {
                    self.primary.insert(fingerprint, remaining);
                }
                return;
            }
            self.collisions.remove(&fingerprint);
            if let Some(remaining) = remaining {
                self.primary.insert(fingerprint, remaining);
            } else {
                self.primary.remove(&fingerprint);
            }
            return;
        }
        if self.primary.get(&fingerprint) == Some(&slot_ref) {
            self.primary.remove(&fingerprint);
        }
    }

    #[cfg(feature = "performance-test")]
    fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + hash_map_allocation_bytes::<u64, SlotRef>(self.primary.capacity())
            + hash_map_allocation_bytes::<u64, Vec<SlotRef>>(self.collisions.capacity())
            + self
                .collisions
                .values()
                .map(|bucket| bucket.capacity() * std::mem::size_of::<SlotRef>())
                .sum::<usize>()
    }
}

/// Fields needed for sorting, filtering, layout and album aggregates. Mutable
/// metadata lives in `QueryIndexes`; descriptions remain sparse dirty patches.
#[derive(Debug, Clone)]
pub struct CacheRecord {
    pub id: ArrayString<64>,
    pub object_type: ObjectType,
    pub timestamp: i64,
    pub width: u32,
    pub height: u32,
    pub size: u64,
    pub thumbhash: Option<Vec<u8>>,
    pub cache_version: u32,
    pub ext: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub path_aliases: Vec<String>,
}

impl CacheRecord {
    pub fn from_abstract_data(data: &AbstractData, timestamp: i64) -> Self {
        let update_at = match data {
            AbstractData::Image(image) => image.object.update_at,
            AbstractData::Video(video) => video.object.update_at,
            AbstractData::Album(album) => album.object.update_at,
        };
        crate::public::structure::object::observe_mutation_timestamp(update_at);
        let (object_type, size) = match data {
            AbstractData::Image(image) => (ObjectType::Image, image.metadata.size),
            AbstractData::Video(video) => (ObjectType::Video, video.metadata.size),
            AbstractData::Album(_) => (ObjectType::Album, 0),
        };
        let exif = data.exif_vec();
        Self {
            id: data.hash(),
            object_type,
            timestamp,
            width: data.width(),
            height: data.height(),
            size,
            thumbhash: data.thumbhash().cloned(),
            cache_version: data.cache_version(),
            ext: data.ext().to_owned(),
            make: exif.and_then(|values| values.get("Make").cloned()),
            model: exif.and_then(|values| values.get("Model").cloned()),
            path_aliases: data
                .alias()
                .iter()
                .map(|alias| alias.file.clone())
                .collect(),
        }
    }

    pub fn reduced(&self, slot_ref: SlotRef) -> ReducedData {
        ReducedData {
            hash: self.id,
            slot_ref: slot_ref.raw(),
            width: self.width,
            height: self.height,
            date: self.timestamp,
        }
    }

    #[cfg(feature = "performance-test")]
    fn estimated_dynamic_bytes(&self) -> usize {
        self.thumbhash.as_ref().map_or(0, Vec::capacity)
            + self.ext.capacity()
            + self.make.as_ref().map_or(0, String::capacity)
            + self.model.as_ref().map_or(0, String::capacity)
            + self.path_aliases.capacity() * std::mem::size_of::<String>()
            + self
                .path_aliases
                .iter()
                .map(String::capacity)
                .sum::<usize>()
    }
}

#[cfg(feature = "performance-test")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeMemoryUsage {
    pub arena_inline_bytes: usize,
    pub record_dynamic_bytes: usize,
    pub id_index_bytes: usize,
    pub order_index_bytes: usize,
    pub query_indexes_bytes: usize,
    pub album_catalog_bytes: usize,
}

#[cfg(feature = "performance-test")]
impl TreeMemoryUsage {
    pub fn total_bytes(self) -> usize {
        self.arena_inline_bytes
            .saturating_add(self.record_dynamic_bytes)
            .saturating_add(self.id_index_bytes)
            .saturating_add(self.order_index_bytes)
            .saturating_add(self.query_indexes_bytes)
            .saturating_add(self.album_catalog_bytes)
    }
}

#[cfg(feature = "performance-test")]
fn hash_map_allocation_bytes<K, V>(capacity: usize) -> usize {
    capacity.saturating_mul(
        std::mem::size_of::<K>()
            .saturating_add(std::mem::size_of::<V>())
            .saturating_add(1),
    )
}

#[cfg(feature = "performance-test")]
fn hash_set_allocation_bytes<T>(capacity: usize) -> usize {
    capacity.saturating_mul(std::mem::size_of::<T>().saturating_add(1))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DenseBitmap {
    words: Vec<u64>,
    len: usize,
}

impl DenseBitmap {
    fn from_words(mut words: Vec<u64>) -> Self {
        while words.last() == Some(&0) {
            words.pop();
        }
        let len = words.iter().map(|word| word.count_ones() as usize).sum();
        Self { words, len }
    }

    pub fn contains(&self, ordinal: u32) -> bool {
        let word = ordinal as usize / 64;
        let bit = ordinal % 64;
        self.words
            .get(word)
            .is_some_and(|value| value & (1_u64 << bit) != 0)
    }

    pub fn set(&mut self, ordinal: u32, value: bool) -> bool {
        let word = ordinal as usize / 64;
        let bit = ordinal % 64;
        if value && word >= self.words.len() {
            self.words.resize(word + 1, 0);
        }
        let Some(entry) = self.words.get_mut(word) else {
            return false;
        };
        let before = *entry & (1_u64 << bit) != 0;
        if value {
            *entry |= 1_u64 << bit;
        } else {
            *entry &= !(1_u64 << bit);
        }
        if before == value {
            return false;
        }
        if value {
            self.len += 1;
        } else {
            self.len -= 1;
        }
        true
    }

    pub fn count(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.words
            .iter()
            .enumerate()
            .flat_map(|(word_index, word)| {
                let mut remaining = *word;
                std::iter::from_fn(move || {
                    if remaining == 0 {
                        return None;
                    }
                    let bit = remaining.trailing_zeros();
                    remaining &= remaining - 1;
                    Some(u32::try_from(word_index).expect("bitmap index") * 64 + bit)
                })
            })
    }

    fn union_with(&mut self, other: &Self, mut changed: impl FnMut(u32)) {
        if self.words.len() < other.words.len() {
            self.words.resize(other.words.len(), 0);
        }
        for (word_index, (word, other_word)) in self.words.iter_mut().zip(&other.words).enumerate()
        {
            let added = *other_word & !*word;
            if added == 0 {
                continue;
            }
            *word |= *other_word;
            self.len += added.count_ones() as usize;
            visit_word_bits(word_index, added, &mut changed);
        }
    }

    fn subtract_with(&mut self, other: &Self, mut changed: impl FnMut(u32)) {
        for (word_index, (word, other_word)) in self.words.iter_mut().zip(&other.words).enumerate()
        {
            let removed = *word & *other_word;
            if removed == 0 {
                continue;
            }
            *word &= !*other_word;
            self.len -= removed.count_ones() as usize;
            visit_word_bits(word_index, removed, &mut changed);
        }
        while self.words.last() == Some(&0) {
            self.words.pop();
        }
    }
}

fn visit_word_bits(word_index: usize, mut bits: u64, visit: &mut impl FnMut(u32)) {
    while bits != 0 {
        let bit = bits.trailing_zeros();
        bits &= bits - 1;
        visit(u32::try_from(word_index).expect("bitmap index") * 64 + bit);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinalSet {
    Sparse(Vec<u32>),
    Dense(DenseBitmap),
}

impl Default for OrdinalSet {
    fn default() -> Self {
        Self::Sparse(Vec::new())
    }
}

impl OrdinalSet {
    pub fn contains(&self, ordinal: u32) -> bool {
        match self {
            Self::Sparse(items) => items.binary_search(&ordinal).is_ok(),
            Self::Dense(bitmap) => bitmap.contains(ordinal),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Sparse(items) => items.len(),
            Self::Dense(bitmap) => bitmap.count(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn max(&self) -> Option<u32> {
        match self {
            Self::Sparse(items) => items.last().copied(),
            Self::Dense(bitmap) => {
                bitmap
                    .words
                    .iter()
                    .rposition(|word| *word != 0)
                    .map(|word_index| {
                        let word = bitmap.words[word_index];
                        u32::try_from(word_index).expect("bitmap index") * 64
                            + (63 - word.leading_zeros())
                    })
            }
        }
    }

    pub fn insert(&mut self, ordinal: u32, universe: usize) -> bool {
        let changed = match self {
            Self::Sparse(items) => match items.binary_search(&ordinal) {
                Ok(_) => false,
                Err(index) => {
                    items.insert(index, ordinal);
                    true
                }
            },
            Self::Dense(bitmap) => bitmap.set(ordinal, true),
        };
        self.rebalance(universe);
        changed
    }

    pub fn remove(&mut self, ordinal: u32, universe: usize) -> bool {
        let changed = match self {
            Self::Sparse(items) => match items.binary_search(&ordinal) {
                Ok(index) => {
                    items.remove(index);
                    true
                }
                Err(_) => false,
            },
            Self::Dense(bitmap) => bitmap.set(ordinal, false),
        };
        self.rebalance(universe);
        changed
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = u32> + '_> {
        match self {
            Self::Sparse(items) => Box::new(items.iter().copied()),
            Self::Dense(bitmap) => Box::new(bitmap.iter()),
        }
    }

    pub fn from_ordinals(ordinals: impl IntoIterator<Item = u32>, universe: usize) -> Self {
        let mut values = ordinals.into_iter().collect::<Vec<_>>();
        values.sort_unstable();
        values.dedup();
        let mut result = Self::Sparse(values);
        result.rebalance(universe);
        result
    }

    pub fn estimated_bytes(&self) -> usize {
        match self {
            Self::Sparse(items) => items.capacity() * std::mem::size_of::<u32>(),
            Self::Dense(bitmap) => bitmap.words.capacity() * std::mem::size_of::<u64>(),
        }
    }

    fn dense_words(&self, universe: usize) -> Vec<u64> {
        let word_count = universe.saturating_add(63) / 64;
        let mut words = vec![0_u64; word_count];
        match self {
            Self::Sparse(items) => {
                for ordinal in items {
                    let ordinal = *ordinal as usize;
                    words[ordinal / 64] |= 1_u64 << (ordinal % 64);
                }
            }
            Self::Dense(bitmap) => {
                let copy_len = bitmap.words.len().min(words.len());
                words[..copy_len].copy_from_slice(&bitmap.words[..copy_len]);
            }
        }
        words
    }

    pub fn subtract(&mut self, removed: &Self, universe: usize) {
        self.subtract_with(removed, universe, |_| {});
    }

    pub fn union_with(&mut self, added: &Self, universe: usize, mut changed: impl FnMut(u32)) {
        match (&mut *self, added) {
            (Self::Sparse(items), Self::Sparse(added)) => {
                let mut next = Vec::with_capacity(items.len().saturating_add(added.len()));
                let (mut left, mut right) = (0, 0);
                while left < items.len() && right < added.len() {
                    match items[left].cmp(&added[right]) {
                        Ordering::Less => {
                            next.push(items[left]);
                            left += 1;
                        }
                        Ordering::Equal => {
                            next.push(items[left]);
                            left += 1;
                            right += 1;
                        }
                        Ordering::Greater => {
                            next.push(added[right]);
                            changed(added[right]);
                            right += 1;
                        }
                    }
                }
                next.extend_from_slice(&items[left..]);
                for value in &added[right..] {
                    next.push(*value);
                    changed(*value);
                }
                *items = next;
            }
            (Self::Sparse(items), Self::Dense(added)) => {
                let previous = std::mem::take(items);
                let mut bitmap = DenseBitmap::default();
                for ordinal in previous {
                    bitmap.set(ordinal, true);
                }
                bitmap.union_with(added, changed);
                *self = Self::Dense(bitmap);
            }
            (Self::Dense(bitmap), Self::Sparse(added)) => {
                for ordinal in added {
                    if bitmap.set(*ordinal, true) {
                        changed(*ordinal);
                    }
                }
            }
            (Self::Dense(bitmap), Self::Dense(added)) => bitmap.union_with(added, changed),
        }
        self.rebalance(universe);
    }

    pub fn subtract_with(&mut self, removed: &Self, universe: usize, mut changed: impl FnMut(u32)) {
        match (&mut *self, removed) {
            (Self::Sparse(items), Self::Sparse(removed)) => {
                let mut next = Vec::with_capacity(items.len());
                let mut removed_index = 0;
                for item in items.iter().copied() {
                    while removed
                        .get(removed_index)
                        .is_some_and(|value| *value < item)
                    {
                        removed_index += 1;
                    }
                    if removed.get(removed_index) != Some(&item) {
                        next.push(item);
                    } else {
                        changed(item);
                    }
                }
                *items = next;
            }
            (Self::Sparse(items), Self::Dense(removed)) => {
                items.retain(|ordinal| {
                    let retain = !removed.contains(*ordinal);
                    if !retain {
                        changed(*ordinal);
                    }
                    retain
                });
            }
            (Self::Dense(bitmap), Self::Sparse(removed)) => {
                for ordinal in removed {
                    if bitmap.set(*ordinal, false) {
                        changed(*ordinal);
                    }
                }
            }
            (Self::Dense(bitmap), Self::Dense(removed)) => {
                bitmap.subtract_with(removed, changed);
            }
        }
        self.rebalance(universe);
    }

    fn subtract_without_rebalance(&mut self, removed: &Self) {
        match (&mut *self, removed) {
            (Self::Sparse(items), Self::Sparse(removed)) => {
                let mut next = Vec::with_capacity(items.len());
                let mut removed_index = 0;
                for item in items.iter().copied() {
                    while removed
                        .get(removed_index)
                        .is_some_and(|value| *value < item)
                    {
                        removed_index += 1;
                    }
                    if removed.get(removed_index) != Some(&item) {
                        next.push(item);
                    }
                }
                *items = next;
            }
            (Self::Sparse(items), Self::Dense(removed)) => {
                items.retain(|ordinal| !removed.contains(*ordinal));
            }
            (Self::Dense(bitmap), Self::Sparse(removed)) => {
                for ordinal in removed {
                    bitmap.set(*ordinal, false);
                }
            }
            (Self::Dense(bitmap), Self::Dense(removed)) => {
                bitmap.subtract_with(removed, |_| {});
            }
        }
    }

    fn rebalance(&mut self, universe: usize) {
        let dense_threshold = universe / 32;
        match self {
            Self::Sparse(items) if items.len() > dense_threshold => {
                let mut bitmap = DenseBitmap::default();
                for ordinal in items.iter().copied() {
                    bitmap.set(ordinal, true);
                }
                *self = Self::Dense(bitmap);
            }
            Self::Dense(bitmap) if bitmap.count() <= dense_threshold => {
                *self = Self::Sparse(bitmap.iter().collect());
            }
            _ => {}
        }
    }
}

/// Compact generational selection used by dirty deltas. Most long-lived
/// galleries have generation 1 for nearly every occupied slot, so only reused
/// slots need an override in addition to the adaptive ordinal bitmap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetSet {
    ordinals: OrdinalSet,
    generation_overrides: Vec<(u32, u32)>,
}

impl TargetSet {
    pub fn from_slot_refs(slot_refs: impl IntoIterator<Item = SlotRef>, universe: usize) -> Self {
        let mut slots = slot_refs.into_iter().collect::<Vec<_>>();
        slots.sort_unstable_by_key(|slot_ref| slot_ref.index());
        slots.dedup_by_key(|slot_ref| slot_ref.index());
        let generation_overrides = slots
            .iter()
            .filter(|slot_ref| slot_ref.generation() != 1)
            .map(|slot_ref| (slot_ref.index(), slot_ref.generation()))
            .collect();
        let ordinals = OrdinalSet::from_ordinals(slots.into_iter().map(SlotRef::index), universe);
        Self {
            ordinals,
            generation_overrides,
        }
    }

    pub fn from_unique_slot_refs(
        slot_refs: impl IntoIterator<Item = SlotRef>,
        universe: usize,
    ) -> Self {
        let mut bitmap = DenseBitmap::default();
        let mut generation_overrides = Vec::new();
        for slot_ref in slot_refs {
            if bitmap.set(slot_ref.index(), true) && slot_ref.generation() != 1 {
                generation_overrides.push((slot_ref.index(), slot_ref.generation()));
            }
        }
        generation_overrides.sort_unstable_by_key(|(ordinal, _)| *ordinal);
        let mut ordinals = OrdinalSet::Dense(bitmap);
        ordinals.rebalance(universe);
        Self {
            ordinals,
            generation_overrides,
        }
    }

    pub fn from_dense_parts(words: Vec<u64>, generation_overrides: Vec<(u32, u32)>) -> Self {
        Self {
            ordinals: OrdinalSet::Dense(DenseBitmap::from_words(words)),
            generation_overrides,
        }
    }

    pub fn dense_parts(&self, universe: usize) -> (Vec<u64>, &[(u32, u32)]) {
        (
            self.ordinals.dense_words(universe),
            &self.generation_overrides,
        )
    }

    pub fn slot_ref_for_ordinal(&self, ordinal: u32) -> Option<SlotRef> {
        if !self.ordinals.contains(ordinal) {
            return None;
        }
        let generation = self
            .generation_overrides
            .binary_search_by_key(&ordinal, |(index, _)| *index)
            .ok()
            .map_or(1, |index| self.generation_overrides[index].1);
        Some(SlotRef::new(ordinal, generation))
    }

    pub fn changed_for_bitmap(
        &self,
        bitmap: &DenseBitmap,
        new_value: bool,
        universe: usize,
    ) -> Self {
        let mut words = self.ordinals.dense_words(universe);
        for (index, word) in words.iter_mut().enumerate() {
            let current = bitmap.words.get(index).copied().unwrap_or(0);
            *word &= if new_value { !current } else { current };
        }
        let mut ordinals = OrdinalSet::Dense(DenseBitmap::from_words(words));
        ordinals.rebalance(universe);
        let generation_overrides = self
            .generation_overrides
            .iter()
            .copied()
            .filter(|(ordinal, _)| ordinals.contains(*ordinal))
            .collect();
        Self {
            ordinals,
            generation_overrides,
        }
    }

    pub fn ordinals(&self) -> &OrdinalSet {
        &self.ordinals
    }

    pub fn len(&self) -> usize {
        self.ordinals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordinals.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = SlotRef> + '_ {
        self.ordinals.iter().map(|ordinal| {
            let generation = self
                .generation_overrides
                .binary_search_by_key(&ordinal, |(index, _)| *index)
                .ok()
                .map_or(1, |index| self.generation_overrides[index].1);
            SlotRef::new(ordinal, generation)
        })
    }

    pub fn contains(&self, slot_ref: SlotRef) -> bool {
        self.ordinals.contains(slot_ref.index())
            && self
                .generation_overrides
                .binary_search_by_key(&slot_ref.index(), |(index, _)| *index)
                .ok()
                .map_or(slot_ref.generation() == 1, |index| {
                    self.generation_overrides[index].1 == slot_ref.generation()
                })
    }

    pub fn is_current(&self, state: &TreeState) -> bool {
        self.iter().all(|slot_ref| state.get(slot_ref).is_some())
    }

    pub fn estimated_bytes(&self) -> usize {
        self.ordinals.estimated_bytes()
            + self.generation_overrides.capacity() * std::mem::size_of::<(u32, u32)>()
    }

    pub fn subtract(&mut self, removed: &Self) {
        if self.generation_overrides.is_empty() && removed.generation_overrides.is_empty() {
            self.ordinals.subtract_without_rebalance(&removed.ordinals);
        } else {
            let matching = removed
                .iter()
                .filter(|slot_ref| self.contains(*slot_ref))
                .map(SlotRef::index)
                .collect::<Vec<_>>();
            let matching = OrdinalSet::Sparse(matching);
            self.ordinals.subtract_without_rebalance(&matching);
        }
        self.generation_overrides
            .retain(|(ordinal, generation)| !removed.contains(SlotRef::new(*ordinal, *generation)));
    }

    pub fn union(&mut self, added: &Self, universe: usize) {
        self.ordinals.union_with(&added.ordinals, universe, |_| {});
        for (ordinal, generation) in &added.generation_overrides {
            match self
                .generation_overrides
                .binary_search_by_key(ordinal, |(index, _)| *index)
            {
                Ok(index) => {
                    self.generation_overrides[index].1 =
                        self.generation_overrides[index].1.max(*generation);
                }
                Err(index) => self
                    .generation_overrides
                    .insert(index, (*ordinal, *generation)),
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct QueryIndexes {
    pub favorite: DenseBitmap,
    pub archived: DenseBitmap,
    pub trashed: DenseBitmap,
    pub has_any_tag: DenseBitmap,
    pub has_any_album: DenseBitmap,
    pub tags: HashMap<String, OrdinalSet>,
    pub albums: HashMap<ArrayString<64>, OrdinalSet>,
    tag_membership_count: Vec<u32>,
    album_membership_count: Vec<u32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FlagPatch {
    pub favorite: Option<bool>,
    pub archived: Option<bool>,
    pub trashed: Option<bool>,
}

impl QueryIndexes {
    #[cfg(feature = "performance-test")]
    fn estimated_bytes(&self) -> usize {
        let bitmap_bytes = [
            &self.favorite,
            &self.archived,
            &self.trashed,
            &self.has_any_tag,
            &self.has_any_album,
        ]
        .into_iter()
        .map(|bitmap| bitmap.words.capacity() * std::mem::size_of::<u64>())
        .sum::<usize>();
        let tag_bytes = hash_map_allocation_bytes::<String, OrdinalSet>(self.tags.capacity())
            + self
                .tags
                .iter()
                .map(|(tag, members)| tag.capacity() + members.estimated_bytes())
                .sum::<usize>();
        let album_bytes =
            hash_map_allocation_bytes::<ArrayString<64>, OrdinalSet>(self.albums.capacity())
                + self
                    .albums
                    .values()
                    .map(OrdinalSet::estimated_bytes)
                    .sum::<usize>();
        std::mem::size_of::<Self>()
            + bitmap_bytes
            + tag_bytes
            + album_bytes
            + self.tag_membership_count.capacity() * std::mem::size_of::<u32>()
            + self.album_membership_count.capacity() * std::mem::size_of::<u32>()
    }

    fn ensure_ordinal(&mut self, ordinal: u32) {
        let len = ordinal as usize + 1;
        if self.tag_membership_count.len() < len {
            self.tag_membership_count.resize(len, 0);
        }
        if self.album_membership_count.len() < len {
            self.album_membership_count.resize(len, 0);
        }
    }

    fn insert_record(&mut self, ordinal: u32, data: &AbstractData, universe: usize) {
        self.ensure_ordinal(ordinal);
        self.favorite.set(ordinal, object_flags(data).0);
        self.archived.set(ordinal, object_flags(data).1);
        self.trashed.set(ordinal, object_flags(data).2);
        for tag in data.tag() {
            self.tags
                .entry(tag.clone())
                .or_default()
                .insert(ordinal, universe);
        }
        self.tag_membership_count[ordinal as usize] =
            u32::try_from(data.tag().len()).unwrap_or(u32::MAX);
        self.has_any_tag.set(ordinal, !data.tag().is_empty());
        if let Some(albums) = data.albums() {
            for album in albums {
                self.albums
                    .entry(*album)
                    .or_default()
                    .insert(ordinal, universe);
            }
            self.album_membership_count[ordinal as usize] =
                u32::try_from(albums.len()).unwrap_or(u32::MAX);
            self.has_any_album.set(ordinal, !albums.is_empty());
        } else {
            self.album_membership_count[ordinal as usize] = 0;
        }
    }

    fn remove_record(&mut self, ordinal: u32, universe: usize) {
        self.favorite.set(ordinal, false);
        self.archived.set(ordinal, false);
        self.trashed.set(ordinal, false);
        self.has_any_tag.set(ordinal, false);
        self.has_any_album.set(ordinal, false);
        self.ensure_ordinal(ordinal);
        self.tag_membership_count[ordinal as usize] = 0;
        self.album_membership_count[ordinal as usize] = 0;
        self.tags.retain(|_, members| {
            members.remove(ordinal, universe);
            !members.is_empty()
        });
        self.albums.retain(|_, members| {
            members.remove(ordinal, universe);
            !members.is_empty()
        });
    }

    fn remove_targets(&mut self, targets: &OrdinalSet, universe: usize) {
        for ordinal in targets.iter() {
            self.favorite.set(ordinal, false);
            self.archived.set(ordinal, false);
            self.trashed.set(ordinal, false);
            self.has_any_tag.set(ordinal, false);
            self.has_any_album.set(ordinal, false);
            self.ensure_ordinal(ordinal);
            self.tag_membership_count[ordinal as usize] = 0;
            self.album_membership_count[ordinal as usize] = 0;
        }
        self.tags.retain(|_, members| {
            members.subtract(targets, universe);
            !members.is_empty()
        });
        self.albums.retain(|_, members| {
            members.subtract(targets, universe);
            !members.is_empty()
        });
    }

    pub fn edit_flags(&mut self, targets: &OrdinalSet, patch: FlagPatch) {
        let apply = |bitmap: &mut DenseBitmap, value| match targets {
            OrdinalSet::Sparse(ordinals) => {
                for ordinal in ordinals {
                    bitmap.set(*ordinal, value);
                }
            }
            OrdinalSet::Dense(targets) if value => bitmap.union_with(targets, |_| {}),
            OrdinalSet::Dense(targets) => bitmap.subtract_with(targets, |_| {}),
        };
        if let Some(value) = patch.favorite {
            apply(&mut self.favorite, value);
        }
        if let Some(value) = patch.archived {
            apply(&mut self.archived, value);
        }
        if let Some(value) = patch.trashed {
            apply(&mut self.trashed, value);
        }
    }

    pub fn edit_tags(
        &mut self,
        targets: &OrdinalSet,
        add: &BTreeSet<String>,
        remove: &BTreeSet<String>,
        universe: usize,
    ) {
        let started = Instant::now();
        if let Some(max_ordinal) = targets.max() {
            self.ensure_ordinal(max_ordinal);
        }
        for tag in add {
            let members = self.tags.entry(tag.clone()).or_default();
            let membership_count = &mut self.tag_membership_count;
            let has_any = &mut self.has_any_tag;
            members.union_with(targets, universe, |ordinal| {
                let count = &mut membership_count[ordinal as usize];
                *count = count.saturating_add(1);
                if *count == 1 {
                    has_any.set(ordinal, true);
                }
            });
        }
        for tag in remove {
            if let Some(members) = self.tags.get_mut(tag) {
                let membership_count = &mut self.tag_membership_count;
                let has_any = &mut self.has_any_tag;
                members.subtract_with(targets, universe, |ordinal| {
                    let count = &mut membership_count[ordinal as usize];
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        has_any.set(ordinal, false);
                    }
                });
            }
        }
        self.tags.retain(|_, members| !members.is_empty());
        crate::perf_timing!(
            "query_indexes.edit_tags.bulk",
            started,
            "Apply tag membership batch"
        );
    }

    pub fn edit_albums(
        &mut self,
        targets: &OrdinalSet,
        add: &BTreeSet<ArrayString<64>>,
        remove: &BTreeSet<ArrayString<64>>,
        universe: usize,
    ) {
        let started = Instant::now();
        if let Some(max_ordinal) = targets.max() {
            self.ensure_ordinal(max_ordinal);
        }
        for album in add {
            let members = self.albums.entry(*album).or_default();
            let membership_count = &mut self.album_membership_count;
            let has_any = &mut self.has_any_album;
            members.union_with(targets, universe, |ordinal| {
                let count = &mut membership_count[ordinal as usize];
                *count = count.saturating_add(1);
                if *count == 1 {
                    has_any.set(ordinal, true);
                }
            });
        }
        for album in remove {
            if let Some(members) = self.albums.get_mut(album) {
                let membership_count = &mut self.album_membership_count;
                let has_any = &mut self.has_any_album;
                members.subtract_with(targets, universe, |ordinal| {
                    let count = &mut membership_count[ordinal as usize];
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        has_any.set(ordinal, false);
                    }
                });
            }
        }
        self.albums.retain(|_, members| !members.is_empty());
        crate::perf_timing!(
            "query_indexes.edit_albums.bulk",
            started,
            "Apply album membership batch"
        );
    }
}

fn object_flags(data: &AbstractData) -> (bool, bool, bool) {
    match data {
        AbstractData::Image(value) => (
            value.object.is_favorite,
            value.object.is_archived,
            value.object.is_trashed,
        ),
        AbstractData::Video(value) => (
            value.object.is_favorite,
            value.object.is_archived,
            value.object.is_trashed,
        ),
        AbstractData::Album(value) => (
            value.object.is_favorite,
            value.object.is_archived,
            value.object.is_trashed,
        ),
    }
}

#[derive(Debug)]
pub struct TreeState {
    pub arena: RecordArena<CacheRecord>,
    pub id_index: IdIndex,
    pub order: Arc<Vec<SlotRef>>,
    pub query: QueryIndexes,
    pub albums: HashMap<ArrayString<64>, AlbumCombined>,
    #[cfg(feature = "performance-test")]
    record_dynamic_bytes: usize,
    structural_epoch: u64,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            arena: RecordArena::default(),
            id_index: IdIndex::default(),
            order: Arc::default(),
            query: QueryIndexes::default(),
            albums: HashMap::default(),
            #[cfg(feature = "performance-test")]
            record_dynamic_bytes: 0,
            structural_epoch: next_structural_epoch(),
        }
    }
}

impl TreeState {
    pub fn edit_cached_album_objects(
        &mut self,
        targets: &TargetSet,
        changed_at: i64,
        mut edit: impl FnMut(&mut ObjectSchema),
    ) {
        let album_ids = targets
            .iter()
            .filter_map(|slot_ref| self.get(slot_ref))
            .filter(|record| record.object_type == ObjectType::Album)
            .map(|record| record.id)
            .collect::<Vec<_>>();
        for album_id in album_ids {
            if let Some(album) = self.albums.get_mut(&album_id) {
                edit(&mut album.object);
                album.object.touch_update_at(changed_at);
            }
        }
    }

    pub fn from_records(records: impl IntoIterator<Item = AbstractData>) -> Self {
        let mut state = Self::default();
        for data in records {
            let timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
            let record = CacheRecord::from_abstract_data(&data, timestamp);
            state.track_record_added(&record);
            let id = record.id;
            let slot_ref = state.arena.allocate(record);
            state.id_index.insert(id.as_str(), slot_ref);
            if let AbstractData::Album(album) = &data {
                state.albums.insert(album.object.id, album.clone());
            }
            let universe = state.arena.capacity();
            state.query.insert_record(slot_ref.index(), &data, universe);
            Arc::make_mut(&mut state.order).push(slot_ref);
        }
        let arena = &state.arena;
        Arc::make_mut(&mut state.order)
            .sort_unstable_by(|left, right| compare_slots(arena, *left, *right));
        state
    }

    pub fn len(&self) -> usize {
        self.arena.len()
    }

    #[cfg(feature = "performance-test")]
    pub fn memory_usage(&self) -> TreeMemoryUsage {
        let arena_inline_bytes = std::mem::size_of::<RecordArena<CacheRecord>>()
            + self.arena.slots.capacity() * std::mem::size_of::<ArenaSlot<CacheRecord>>()
            + self.arena.free.capacity() * std::mem::size_of::<u32>()
            + std::mem::size_of::<u64>();
        let order_index_bytes = std::mem::size_of::<Arc<Vec<SlotRef>>>()
            + std::mem::size_of::<Vec<SlotRef>>()
            + 2 * std::mem::size_of::<usize>()
            + self.order.capacity() * std::mem::size_of::<SlotRef>();
        let album_catalog_bytes = std::mem::size_of::<HashMap<ArrayString<64>, AlbumCombined>>()
            + hash_map_allocation_bytes::<ArrayString<64>, AlbumCombined>(self.albums.capacity())
            + self.albums.values().map(album_dynamic_bytes).sum::<usize>();
        TreeMemoryUsage {
            arena_inline_bytes,
            record_dynamic_bytes: self.record_dynamic_bytes,
            id_index_bytes: self.id_index.estimated_bytes(),
            order_index_bytes,
            query_indexes_bytes: self.query.estimated_bytes(),
            album_catalog_bytes,
        }
    }

    fn track_record_added(&mut self, record: &CacheRecord) {
        #[cfg(feature = "performance-test")]
        {
            self.record_dynamic_bytes = self
                .record_dynamic_bytes
                .saturating_add(record.estimated_dynamic_bytes());
        }
        #[cfg(not(feature = "performance-test"))]
        let _ = record;
    }

    fn track_record_removed(&mut self, record: &CacheRecord) {
        #[cfg(feature = "performance-test")]
        {
            self.record_dynamic_bytes = self
                .record_dynamic_bytes
                .saturating_sub(record.estimated_dynamic_bytes());
        }
        #[cfg(not(feature = "performance-test"))]
        let _ = record;
    }

    pub fn structural_epoch(&self) -> u64 {
        self.structural_epoch
    }

    fn bump_structural_epoch(&mut self) {
        self.structural_epoch = next_structural_epoch();
    }

    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }

    pub fn get(&self, slot_ref: SlotRef) -> Option<&CacheRecord> {
        self.arena.get(slot_ref)
    }

    pub fn find(&self, id: &str) -> Option<SlotRef> {
        self.id_index.find(id, &self.arena)
    }

    pub fn slot_for_ordinal(&self, ordinal: u32) -> Option<SlotRef> {
        self.arena.slot_at_ordinal(ordinal)
    }

    pub fn media_targets(&self, targets: &TargetSet) -> TargetSet {
        TargetSet::from_unique_slot_refs(
            targets.iter().filter(|slot_ref| {
                self.get(*slot_ref)
                    .is_some_and(|record| record.object_type != ObjectType::Album)
            }),
            self.arena.capacity(),
        )
    }

    pub fn reduced_ordered(&self) -> Vec<ReducedData> {
        self.order
            .iter()
            .filter_map(|slot_ref| self.get(*slot_ref).map(|record| record.reduced(*slot_ref)))
            .collect()
    }

    pub fn insert(&mut self, data: &AbstractData) -> SlotRef {
        let timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
        let record = CacheRecord::from_abstract_data(data, timestamp);
        self.track_record_added(&record);
        let id = record.id;
        let slot_ref = self.arena.allocate(record);
        self.id_index.insert(id.as_str(), slot_ref);
        if let AbstractData::Album(album) = data {
            self.albums.insert(album.object.id, album.clone());
        }
        let universe = self.arena.capacity();
        self.query.insert_record(slot_ref.index(), data, universe);

        let position = self
            .order
            .binary_search_by(|probe| compare_slots(&self.arena, *probe, slot_ref))
            .unwrap_or_else(std::convert::identity);
        Arc::make_mut(&mut self.order).insert(position, slot_ref);
        self.bump_structural_epoch();
        slot_ref
    }

    pub fn remove(&mut self, slot_ref: SlotRef) -> Option<CacheRecord> {
        let id = self.arena.get(slot_ref)?.id;
        self.id_index.remove(id.as_str(), slot_ref);
        self.query
            .remove_record(slot_ref.index(), self.arena.capacity());
        self.albums.remove(&id);
        Arc::make_mut(&mut self.order).retain(|candidate| *candidate != slot_ref);
        let removed = self.arena.remove(slot_ref);
        if let Some(record) = &removed {
            self.track_record_removed(record);
            self.bump_structural_epoch();
        }
        removed
    }

    pub fn remove_targets(&mut self, targets: &TargetSet) {
        if targets.is_empty() {
            return;
        }
        let universe = self.arena.capacity();
        self.query.remove_targets(targets.ordinals(), universe);
        for slot_ref in targets.iter() {
            let Some(id) = self.arena.get(slot_ref).map(|record| record.id) else {
                continue;
            };
            self.id_index.remove(id.as_str(), slot_ref);
            self.albums.remove(&id);
            if let Some(record) = self.arena.remove(slot_ref) {
                self.track_record_removed(&record);
            }
        }
        let next_order = self
            .order
            .iter()
            .copied()
            .filter(|slot_ref| !targets.contains(*slot_ref))
            .collect();
        self.order = Arc::new(next_order);
        self.bump_structural_epoch();
    }

    pub fn replace_static(&mut self, slot_ref: SlotRef, data: &AbstractData) -> Option<()> {
        let old = self.arena.get(slot_ref)?.clone();
        let new_timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
        let new = CacheRecord::from_abstract_data(data, new_timestamp);
        if old.id != new.id {
            return None;
        }
        self.query
            .remove_record(slot_ref.index(), self.arena.capacity());
        self.query
            .insert_record(slot_ref.index(), data, self.arena.capacity());
        if let AbstractData::Album(album) = data {
            self.albums.insert(album.object.id, album.clone());
        }
        #[cfg(feature = "performance-test")]
        {
            self.record_dynamic_bytes = self
                .record_dynamic_bytes
                .saturating_sub(old.estimated_dynamic_bytes())
                .saturating_add(new.estimated_dynamic_bytes());
        }
        *self.arena.get_mut(slot_ref)? = new;
        if old.timestamp != new_timestamp {
            let mut order = self
                .order
                .iter()
                .copied()
                .filter(|item| *item != slot_ref)
                .collect();
            sort_and_merge(&self.arena, &mut order, vec![slot_ref]);
            self.order = Arc::new(order);
            self.bump_structural_epoch();
        }
        Some(())
    }

    /// Apply a physical import/delete/reindex batch with one order-index
    /// rebuild. Records remain in stable arena slots; only removed/rekeyed
    /// identities are filtered before a deterministic sorted merge.
    pub fn apply_batch(
        &mut self,
        insert_list: &[AbstractData],
        remove_ids: &HashSet<ArrayString<64>>,
    ) {
        let removed_slots = remove_ids
            .iter()
            .filter_map(|id| self.find(id.as_str()))
            .collect::<HashSet<_>>();
        let unique_insert_ids = insert_list
            .iter()
            .map(AbstractData::hash)
            .collect::<HashSet<_>>();
        let additions = unique_insert_ids
            .iter()
            .filter(|id| remove_ids.contains(*id) || self.find(id.as_str()).is_none())
            .count();
        let final_universe = self.arena.future_capacity(additions, removed_slots.len());
        let reset_slots = TargetSet::from_slot_refs(
            removed_slots.iter().copied().chain(
                unique_insert_ids
                    .iter()
                    .filter(|id| !remove_ids.contains(*id))
                    .filter_map(|id| self.find(id.as_str())),
            ),
            self.arena.capacity(),
        );
        self.query
            .remove_targets(reset_slots.ordinals(), final_universe);

        for slot_ref in &removed_slots {
            let Some(id) = self.arena.get(*slot_ref).map(|record| record.id) else {
                continue;
            };
            self.id_index.remove(id.as_str(), *slot_ref);
            self.albums.remove(&id);
            if let Some(record) = self.arena.remove(*slot_ref) {
                self.track_record_removed(&record);
            }
        }

        let mut rekeyed = HashSet::new();
        let mut additions = Vec::new();
        for data in insert_list {
            let id = data.hash();
            let timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
            let next_record = CacheRecord::from_abstract_data(data, timestamp);
            if let Some(slot_ref) = self.find(id.as_str()) {
                let Some(old_timestamp) = self.get(slot_ref).map(|record| record.timestamp) else {
                    continue;
                };
                #[cfg(feature = "performance-test")]
                let old_dynamic_bytes = self
                    .get(slot_ref)
                    .map_or(0, CacheRecord::estimated_dynamic_bytes);
                self.query
                    .insert_record(slot_ref.index(), data, final_universe);
                if let AbstractData::Album(album) = data {
                    self.albums.insert(album.object.id, album.clone());
                }
                #[cfg(feature = "performance-test")]
                {
                    self.record_dynamic_bytes = self
                        .record_dynamic_bytes
                        .saturating_sub(old_dynamic_bytes)
                        .saturating_add(next_record.estimated_dynamic_bytes());
                }
                if let Some(record) = self.arena.get_mut(slot_ref) {
                    *record = next_record;
                }
                if old_timestamp != timestamp {
                    rekeyed.insert(slot_ref);
                    additions.push(slot_ref);
                }
            } else {
                self.track_record_added(&next_record);
                let slot_ref = self.arena.allocate(next_record);
                self.id_index.insert(id.as_str(), slot_ref);
                self.query
                    .insert_record(slot_ref.index(), data, final_universe);
                if let AbstractData::Album(album) = data {
                    self.albums.insert(album.object.id, album.clone());
                }
                additions.push(slot_ref);
            }
        }

        let structural_changed = !removed_slots.is_empty() || !additions.is_empty();
        if !removed_slots.is_empty() || !rekeyed.is_empty() || !additions.is_empty() {
            additions.sort_unstable();
            additions.dedup();
            let mut next_order = self
                .order
                .iter()
                .copied()
                .filter(|slot_ref| !removed_slots.contains(slot_ref) && !rekeyed.contains(slot_ref))
                .collect::<Vec<_>>();
            sort_and_merge(&self.arena, &mut next_order, additions);
            self.order = Arc::new(next_order);
        }
        if structural_changed {
            self.bump_structural_epoch();
        }
    }

    pub fn matches(
        &self,
        slot_ref: SlotRef,
        expression: &Expression,
        hidden_metadata_album: Option<ArrayString<64>>,
    ) -> bool {
        let Some(record) = self.get(slot_ref) else {
            return false;
        };
        self.compile_expression(expression, hidden_metadata_album)
            .matches(record, slot_ref.index())
    }

    pub fn compile_expression<'a>(
        &'a self,
        expression: &Expression,
        hidden_metadata_album: Option<ArrayString<64>>,
    ) -> CompiledExpression<'a> {
        CompiledExpression::new(expression, &self.query, hidden_metadata_album)
    }

    pub fn edit_album_memberships(
        &mut self,
        targets: &OrdinalSet,
        add: &BTreeSet<ArrayString<64>>,
        remove: &BTreeSet<ArrayString<64>>,
        changed_at: i64,
    ) -> Vec<AlbumCombined> {
        let mut updated = HashMap::<ArrayString<64>, (AlbumCombined, bool)>::new();
        for album_id in add.union(remove) {
            if let Some(album) = self.albums.get(album_id) {
                updated.insert(*album_id, (album.clone(), false));
            }
        }

        for album_id in add {
            let existing = self.query.albums.get(album_id);
            let Some((album, _)) = updated.get_mut(album_id) else {
                continue;
            };
            let mut cover_candidate = None::<(i64, ArrayString<64>, Option<Vec<u8>>, u32)>;
            for ordinal in targets.iter() {
                if existing.is_some_and(|members| members.contains(ordinal))
                    || self.query.trashed.contains(ordinal)
                {
                    continue;
                }
                let Some(record) = self
                    .slot_for_ordinal(ordinal)
                    .and_then(|slot_ref| self.get(slot_ref))
                    .filter(|record| record.object_type != ObjectType::Album)
                else {
                    continue;
                };
                album.metadata.item_count = album.metadata.item_count.saturating_add(1);
                album.metadata.item_size = album.metadata.item_size.saturating_add(record.size);
                album.metadata.start_time = Some(
                    album
                        .metadata
                        .start_time
                        .map_or(record.timestamp, |value| value.min(record.timestamp)),
                );
                album.metadata.end_time = Some(
                    album
                        .metadata
                        .end_time
                        .map_or(record.timestamp, |value| value.max(record.timestamp)),
                );
                if album.metadata.cover.is_none()
                    && cover_candidate
                        .as_ref()
                        .is_none_or(|(timestamp, id, _, _)| {
                            record.timestamp > *timestamp
                                || (record.timestamp == *timestamp && record.id < *id)
                        })
                {
                    cover_candidate = Some((
                        record.timestamp,
                        record.id,
                        record.thumbhash.clone(),
                        record.cache_version,
                    ));
                }
            }
            if album.metadata.cover.is_none()
                && let Some((_, id, thumbhash, cache_version)) = cover_candidate
            {
                album.metadata.cover = Some(id);
                album.object.thumbhash = thumbhash;
                album.object.cache_version = cache_version;
            }
        }

        for album_id in remove {
            let existing = self.query.albums.get(album_id);
            let Some((album, rebuild)) = updated.get_mut(album_id) else {
                continue;
            };
            for ordinal in targets.iter() {
                if !existing.is_some_and(|members| members.contains(ordinal))
                    || self.query.trashed.contains(ordinal)
                {
                    continue;
                }
                let Some(record) = self
                    .slot_for_ordinal(ordinal)
                    .and_then(|slot_ref| self.get(slot_ref))
                    .filter(|record| record.object_type != ObjectType::Album)
                else {
                    continue;
                };
                album.metadata.item_count = album.metadata.item_count.saturating_sub(1);
                album.metadata.item_size = album.metadata.item_size.saturating_sub(record.size);
                *rebuild |= album.metadata.cover == Some(record.id)
                    || album.metadata.start_time == Some(record.timestamp)
                    || album.metadata.end_time == Some(record.timestamp);
            }
        }

        let universe = self.arena.capacity();
        self.query.edit_albums(targets, add, remove, universe);
        let affected = updated.keys().copied().collect::<Vec<_>>();
        let mut patches = Vec::with_capacity(affected.len());
        for album_id in affected {
            let (mut album, rebuild) = updated.remove(&album_id).expect("affected album");
            if rebuild {
                if let Some(album) = self.refresh_album_aggregate(album_id, changed_at) {
                    patches.push(album);
                }
            } else {
                album.metadata.last_modified_time = changed_at;
                album.object.touch_update_at(changed_at);
                self.albums.insert(album_id, album.clone());
                patches.push(album);
            }
        }
        patches
    }

    pub fn edit_flags_and_refresh(
        &mut self,
        targets: &OrdinalSet,
        patch: FlagPatch,
        changed_at: i64,
    ) -> Vec<AlbumCombined> {
        let Some(trashed) = patch.trashed else {
            self.query.edit_flags(targets, patch);
            return Vec::new();
        };
        let affected = self
            .query
            .albums
            .iter()
            .filter(|(_, members)| targets.iter().any(|ordinal| members.contains(ordinal)))
            .map(|(album_id, _)| *album_id)
            .collect::<Vec<_>>();
        let mut updated = affected
            .iter()
            .filter_map(|album_id| {
                self.albums
                    .get(album_id)
                    .cloned()
                    .map(|album| (*album_id, (album, false)))
            })
            .collect::<HashMap<_, _>>();

        for album_id in &affected {
            let Some(members) = self.query.albums.get(album_id) else {
                continue;
            };
            let Some((album, rebuild)) = updated.get_mut(album_id) else {
                continue;
            };
            let mut cover_candidate = None::<(i64, ArrayString<64>, Option<Vec<u8>>, u32)>;
            for ordinal in targets.iter() {
                if !members.contains(ordinal) || self.query.trashed.contains(ordinal) == trashed {
                    continue;
                }
                let Some(record) = self
                    .slot_for_ordinal(ordinal)
                    .and_then(|slot_ref| self.get(slot_ref))
                    .filter(|record| record.object_type != ObjectType::Album)
                else {
                    continue;
                };
                if trashed {
                    album.metadata.item_count = album.metadata.item_count.saturating_sub(1);
                    album.metadata.item_size = album.metadata.item_size.saturating_sub(record.size);
                    *rebuild |= album.metadata.cover == Some(record.id)
                        || album.metadata.start_time == Some(record.timestamp)
                        || album.metadata.end_time == Some(record.timestamp);
                } else {
                    album.metadata.item_count = album.metadata.item_count.saturating_add(1);
                    album.metadata.item_size = album.metadata.item_size.saturating_add(record.size);
                    album.metadata.start_time = Some(
                        album
                            .metadata
                            .start_time
                            .map_or(record.timestamp, |value| value.min(record.timestamp)),
                    );
                    album.metadata.end_time = Some(
                        album
                            .metadata
                            .end_time
                            .map_or(record.timestamp, |value| value.max(record.timestamp)),
                    );
                    if album.metadata.cover.is_none()
                        && cover_candidate
                            .as_ref()
                            .is_none_or(|(timestamp, id, _, _)| {
                                record.timestamp > *timestamp
                                    || (record.timestamp == *timestamp && record.id < *id)
                            })
                    {
                        cover_candidate = Some((
                            record.timestamp,
                            record.id,
                            record.thumbhash.clone(),
                            record.cache_version,
                        ));
                    }
                }
            }
            if !trashed
                && album.metadata.cover.is_none()
                && let Some((_, id, thumbhash, cache_version)) = cover_candidate
            {
                album.metadata.cover = Some(id);
                album.object.thumbhash = thumbhash;
                album.object.cache_version = cache_version;
            }
        }

        self.query.edit_flags(targets, patch);
        let mut patches = Vec::with_capacity(updated.len());
        for album_id in affected {
            let Some((mut album, rebuild)) = updated.remove(&album_id) else {
                continue;
            };
            if rebuild {
                if let Some(album) = self.refresh_album_aggregate(album_id, changed_at) {
                    patches.push(album);
                }
            } else {
                album.metadata.last_modified_time = changed_at;
                album.object.touch_update_at(changed_at);
                self.albums.insert(album_id, album.clone());
                patches.push(album);
            }
        }
        patches
    }

    pub fn refresh_album_aggregate(
        &mut self,
        album_id: ArrayString<64>,
        changed_at: i64,
    ) -> Option<AlbumCombined> {
        let album = self.album_aggregate_excluding(album_id, &TargetSet::default(), changed_at)?;
        self.albums.insert(album_id, album.clone());
        Some(album)
    }

    pub fn album_aggregate_excluding(
        &self,
        album_id: ArrayString<64>,
        excluded: &TargetSet,
        changed_at: i64,
    ) -> Option<AlbumCombined> {
        let mut album = self.albums.get(&album_id)?.clone();
        let current_cover = album.metadata.cover;
        let mut item_count = 0_usize;
        let mut item_size = 0_u64;
        let mut start_time = None::<i64>;
        let mut end_time = None::<i64>;
        let mut cover_is_member = false;
        let mut cover_thumbhash = None;
        let mut cover_cache_version = 0;
        let mut newest = None::<&CacheRecord>;
        for record in self
            .query
            .albums
            .get(&album_id)
            .into_iter()
            .flat_map(|members| members.iter())
            .filter(|ordinal| !self.query.trashed.contains(*ordinal))
            .filter_map(|ordinal| self.slot_for_ordinal(ordinal))
            .filter(|slot_ref| !excluded.contains(*slot_ref))
            .filter_map(|slot_ref| self.get(slot_ref))
            .filter(|record| record.object_type != ObjectType::Album)
        {
            item_count += 1;
            item_size = item_size.saturating_add(record.size);
            start_time =
                Some(start_time.map_or(record.timestamp, |value| value.min(record.timestamp)));
            end_time = Some(end_time.map_or(record.timestamp, |value| value.max(record.timestamp)));
            if current_cover == Some(record.id) {
                cover_is_member = true;
                cover_thumbhash.clone_from(&record.thumbhash);
                cover_cache_version = record.cache_version;
            }
            if newest.is_none_or(|candidate| {
                record.timestamp > candidate.timestamp
                    || (record.timestamp == candidate.timestamp && record.id < candidate.id)
            }) {
                newest = Some(record);
            }
        }
        album.metadata.item_count = item_count;
        album.metadata.item_size = item_size;
        album.metadata.start_time = start_time;
        album.metadata.end_time = end_time;
        album.metadata.last_modified_time = changed_at;
        album.object.touch_update_at(changed_at);
        if item_count == 0 {
            album.metadata.cover = None;
            album.object.thumbhash = None;
            album.object.cache_version = 0;
        } else if cover_is_member {
            album.object.thumbhash = cover_thumbhash;
            album.object.cache_version = cover_cache_version;
        } else if let Some(record) = newest {
            album.metadata.cover = Some(record.id);
            album.object.thumbhash.clone_from(&record.thumbhash);
            album.object.cache_version = record.cache_version;
        }
        Some(album)
    }

    /// Recalculate an album as if one stable slot already contained `data`.
    /// This lets media publication persist the object and every aggregate in a
    /// single database transaction before mutating the in-memory tree.
    pub fn album_aggregate_with_override(
        &self,
        album_id: ArrayString<64>,
        override_slot: SlotRef,
        data: &AbstractData,
        changed_at: i64,
    ) -> Option<AlbumCombined> {
        let mut album = self.albums.get(&album_id)?.clone();
        let current_cover = album.metadata.cover;
        let override_record = CacheRecord::from_abstract_data(
            data,
            data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST),
        );
        let override_is_member = data
            .albums()
            .is_some_and(|albums| albums.contains(&album_id));
        let override_is_trashed = match data {
            AbstractData::Image(image) => image.object.is_trashed,
            AbstractData::Video(video) => video.object.is_trashed,
            AbstractData::Album(album) => album.object.is_trashed,
        };
        let mut saw_override = false;
        let mut records = Vec::<CacheRecord>::new();
        for slot_ref in self
            .query
            .albums
            .get(&album_id)
            .into_iter()
            .flat_map(|members| members.iter())
            .filter_map(|ordinal| self.slot_for_ordinal(ordinal))
        {
            if slot_ref == override_slot {
                saw_override = true;
                if override_is_member && !override_is_trashed {
                    records.push(override_record.clone());
                }
                continue;
            }
            if self.query.trashed.contains(slot_ref.index()) {
                continue;
            }
            if let Some(record) = self.get(slot_ref) {
                records.push(record.clone());
            }
        }
        if override_is_member && !override_is_trashed && !saw_override {
            records.push(override_record);
        }
        let mut item_count = 0_usize;
        let mut item_size = 0_u64;
        let mut start_time = None::<i64>;
        let mut end_time = None::<i64>;
        let mut cover_is_member = false;
        let mut cover_thumbhash = None;
        let mut cover_cache_version = 0;
        let mut newest = None::<CacheRecord>;

        for record in &records {
            if record.object_type == ObjectType::Album {
                continue;
            }
            item_count += 1;
            item_size = item_size.saturating_add(record.size);
            start_time =
                Some(start_time.map_or(record.timestamp, |value| value.min(record.timestamp)));
            end_time = Some(end_time.map_or(record.timestamp, |value| value.max(record.timestamp)));
            if current_cover == Some(record.id) {
                cover_is_member = true;
                cover_thumbhash.clone_from(&record.thumbhash);
                cover_cache_version = record.cache_version;
            }
            if newest.as_ref().is_none_or(|candidate| {
                record.timestamp > candidate.timestamp
                    || (record.timestamp == candidate.timestamp && record.id < candidate.id)
            }) {
                newest = Some(record.clone());
            }
        }

        album.metadata.item_count = item_count;
        album.metadata.item_size = item_size;
        album.metadata.start_time = start_time;
        album.metadata.end_time = end_time;
        album.metadata.last_modified_time = changed_at;
        album.object.touch_update_at(changed_at);
        if item_count == 0 {
            album.metadata.cover = None;
            album.object.thumbhash = None;
            album.object.cache_version = 0;
        } else if cover_is_member {
            album.object.thumbhash = cover_thumbhash;
            album.object.cache_version = cover_cache_version;
        } else if let Some(record) = newest {
            album.metadata.cover = Some(record.id);
            album.object.thumbhash = record.thumbhash;
            album.object.cache_version = record.cache_version;
        }
        Some(album)
    }
}

#[cfg(feature = "performance-test")]
fn album_dynamic_bytes(album: &AlbumCombined) -> usize {
    let object = &album.object;
    let metadata = &album.metadata;
    let tags = hash_set_allocation_bytes::<String>(object.tags.capacity())
        + object.tags.iter().map(String::capacity).sum::<usize>();
    let shares = hash_map_allocation_bytes::<
        ArrayString<64>,
        crate::public::structure::album::share::Share,
    >(metadata.share_list.capacity())
        + metadata
            .share_list
            .values()
            .map(|share| {
                share.description.capacity() + share.password.as_ref().map_or(0, String::capacity)
            })
            .sum::<usize>();
    object.thumbhash.as_ref().map_or(0, Vec::capacity)
        + object.description.as_ref().map_or(0, String::capacity)
        + tags
        + metadata.title.as_ref().map_or(0, String::capacity)
        + shares
}

fn compare_slots(arena: &RecordArena<CacheRecord>, left: SlotRef, right: SlotRef) -> Ordering {
    match (arena.get(left), arena.get(right)) {
        (Some(left), Some(right)) => right
            .timestamp
            .cmp(&left.timestamp)
            .then_with(|| left.id.cmp(&right.id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.cmp(&right),
    }
}

pub fn sort_and_merge(
    arena: &RecordArena<CacheRecord>,
    existing: &mut Vec<SlotRef>,
    mut additions: Vec<SlotRef>,
) {
    additions.sort_unstable_by(|left, right| compare_slots(arena, *left, *right));
    let mut merged = Vec::with_capacity(existing.len() + additions.len());
    let (mut left, mut right) = (0, 0);
    while left < existing.len() && right < additions.len() {
        if compare_slots(arena, existing[left], additions[right]) != Ordering::Greater {
            merged.push(existing[left]);
            left += 1;
        } else {
            merged.push(additions[right]);
            right += 1;
        }
    }
    merged.extend_from_slice(&existing[left..]);
    merged.extend_from_slice(&additions[right..]);
    *existing = merged;
}

#[derive(Debug)]
pub struct CompiledExpression<'a>(CompiledExpressionNode<'a>);

#[derive(Debug)]
enum CompiledExpressionNode<'a> {
    Constant(bool),
    Or(Vec<Self>),
    And(Vec<Self>),
    Not(Box<Self>),
    Membership(Option<&'a OrdinalSet>),
    Bitmap(&'a DenseBitmap, bool),
    ExtType {
        image: bool,
        video: bool,
        album: bool,
    },
    Ext(String),
    Model(CompiledStringFilter),
    Make(CompiledStringFilter),
    Path(String),
    HiddenAlbumExists(bool),
    Any {
        needle: String,
        tag_members: Option<&'a OrdinalSet>,
        include_paths: bool,
        include_album_type: bool,
    },
}

#[derive(Debug)]
enum CompiledStringFilter {
    Contains(String),
    Exists(bool),
}

impl CompiledStringFilter {
    fn new(filter: &FilterValue) -> Self {
        match filter {
            FilterValue::Value(needle) => Self::Contains(needle.to_ascii_lowercase()),
            FilterValue::Exists(exists) => Self::Exists(*exists),
        }
    }

    fn matches(&self, value: Option<&str>) -> bool {
        match self {
            Self::Contains(needle) => {
                value.is_some_and(|value| contains_ascii_lowercase(value, needle))
            }
            Self::Exists(exists) => value.is_some() == *exists,
        }
    }
}

impl<'a> CompiledExpression<'a> {
    fn new(
        expression: &Expression,
        indexes: &'a QueryIndexes,
        hidden_metadata_album: Option<ArrayString<64>>,
    ) -> Self {
        Self(CompiledExpressionNode::new(
            expression,
            indexes,
            hidden_metadata_album,
        ))
    }

    pub fn matches(&self, record: &CacheRecord, ordinal: u32) -> bool {
        self.0.matches(record, ordinal)
    }
}

impl<'a> CompiledExpressionNode<'a> {
    fn new(
        expression: &Expression,
        indexes: &'a QueryIndexes,
        hidden_metadata_album: Option<ArrayString<64>>,
    ) -> Self {
        match expression {
            Expression::Or(expressions) => Self::Or(
                expressions
                    .iter()
                    .map(|expression| Self::new(expression, indexes, hidden_metadata_album))
                    .collect(),
            ),
            Expression::And(expressions) => Self::And(
                expressions
                    .iter()
                    .map(|expression| Self::new(expression, indexes, hidden_metadata_album))
                    .collect(),
            ),
            Expression::Not(expression) => Self::Not(Box::new(Self::new(
                expression,
                indexes,
                hidden_metadata_album,
            ))),
            Expression::Tag(_) if hidden_metadata_album.is_some() => Self::Constant(false),
            Expression::Tag(FilterValue::Value(tag)) => Self::Membership(indexes.tags.get(tag)),
            Expression::Tag(FilterValue::Exists(exists)) => {
                Self::Bitmap(&indexes.has_any_tag, *exists)
            }
            Expression::Favorite(value) => Self::Bitmap(&indexes.favorite, *value),
            Expression::Archived(value) => Self::Bitmap(&indexes.archived, *value),
            Expression::Trashed(value) => Self::Bitmap(&indexes.trashed, *value),
            Expression::ExtType(ext_type) => Self::ExtType {
                image: ext_type.contains("image"),
                video: ext_type.contains("video"),
                album: hidden_metadata_album.is_none() && ext_type.contains("album"),
            },
            Expression::Ext(ext) => Self::Ext(ext.to_ascii_lowercase()),
            Expression::Model(filter) => Self::Model(CompiledStringFilter::new(filter)),
            Expression::Make(filter) => Self::Make(CompiledStringFilter::new(filter)),
            Expression::Path(_) if hidden_metadata_album.is_some() => Self::Constant(false),
            Expression::Path(path) => Self::Path(path.to_ascii_lowercase()),
            Expression::Album(AlbumFilterValue::Value(album_id))
                if hidden_metadata_album.is_some_and(|allowed| allowed != *album_id) =>
            {
                Self::Constant(false)
            }
            Expression::Album(AlbumFilterValue::Value(album_id)) => {
                Self::Membership(indexes.albums.get(album_id))
            }
            Expression::Album(AlbumFilterValue::Exists(exists))
                if hidden_metadata_album.is_some() =>
            {
                Self::HiddenAlbumExists(*exists)
            }
            Expression::Album(AlbumFilterValue::Exists(exists)) => {
                Self::Bitmap(&indexes.has_any_album, *exists)
            }
            Expression::Any(value) => Self::Any {
                needle: value.to_ascii_lowercase(),
                tag_members: hidden_metadata_album
                    .is_none()
                    .then(|| indexes.tags.get(value))
                    .flatten(),
                include_paths: hidden_metadata_album.is_none(),
                include_album_type: hidden_metadata_album.is_none(),
            },
        }
    }

    fn matches(&self, record: &CacheRecord, ordinal: u32) -> bool {
        match self {
            Self::Constant(value) => *value,
            Self::Or(expressions) => expressions
                .iter()
                .any(|expression| expression.matches(record, ordinal)),
            Self::And(expressions) => expressions
                .iter()
                .all(|expression| expression.matches(record, ordinal)),
            Self::Not(expression) => !expression.matches(record, ordinal),
            Self::Membership(members) => members.is_some_and(|members| members.contains(ordinal)),
            Self::Bitmap(bitmap, value) => bitmap.contains(ordinal) == *value,
            Self::ExtType {
                image,
                video,
                album,
            } => match record.object_type {
                ObjectType::Image => *image,
                ObjectType::Video => *video,
                ObjectType::Album => *album,
            },
            Self::Ext(needle) => contains_ascii_lowercase(&record.ext, needle),
            Self::Model(filter) => filter.matches(record.model.as_deref()),
            Self::Make(filter) => filter.matches(record.make.as_deref()),
            Self::Path(needle) => record
                .path_aliases
                .iter()
                .any(|path| contains_ascii_lowercase(path, needle)),
            Self::HiddenAlbumExists(exists) => record.object_type != ObjectType::Album && *exists,
            Self::Any {
                needle,
                tag_members,
                include_paths,
                include_album_type,
            } => {
                contains_ascii_lowercase(record.id.as_str(), needle)
                    || contains_ascii_lowercase(&record.ext, needle)
                    || record
                        .make
                        .as_deref()
                        .is_some_and(|value| contains_ascii_lowercase(value, needle))
                    || record
                        .model
                        .as_deref()
                        .is_some_and(|value| contains_ascii_lowercase(value, needle))
                    || match record.object_type {
                        ObjectType::Image => "image".contains(needle),
                        ObjectType::Video => "video".contains(needle),
                        ObjectType::Album => *include_album_type && "album".contains(needle),
                    }
                    || tag_members.is_some_and(|members| members.contains(ordinal))
                    || (*include_paths
                        && record
                            .path_aliases
                            .iter()
                            .any(|path| contains_ascii_lowercase(path, needle)))
            }
        }
    }
}

fn contains_ascii_lowercase(value: &str, lowercase_needle: &str) -> bool {
    let needle = lowercase_needle.as_bytes();
    if needle.is_empty() {
        return true;
    }
    value.as_bytes().windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(value, needle)| value.to_ascii_lowercase() == *needle)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::structure::album::Album;

    fn cache_record(id: &str, timestamp: i64) -> CacheRecord {
        CacheRecord {
            id: ArrayString::<64>::from(id).unwrap(),
            object_type: ObjectType::Image,
            timestamp,
            width: 1,
            height: 1,
            size: 1,
            thumbhash: None,
            cache_version: 0,
            ext: "jpg".to_owned(),
            make: None,
            model: None,
            path_aliases: Vec::new(),
        }
    }

    #[test]
    fn arena_reuse_invalidates_old_generation() {
        let mut arena = RecordArena::default();
        let first = arena.allocate("first");
        assert_eq!(arena.remove(first), Some("first"));
        let second = arena.allocate("second");
        assert_eq!(first.index(), second.index());
        assert_ne!(first.generation(), second.generation());
        assert!(arena.get(first).is_none());
        assert_eq!(arena.get(second), Some(&"second"));
    }

    #[test]
    fn ordinal_set_switches_between_sparse_and_dense() {
        let mut set = OrdinalSet::default();
        for value in 0..5 {
            set.insert(value, 64);
        }
        assert!(matches!(set, OrdinalSet::Dense(_)));
        for value in 0..4 {
            set.remove(value, 64);
        }
        assert!(matches!(set, OrdinalSet::Sparse(_)));
        assert!(set.contains(4));
    }

    #[test]
    fn ordinal_set_batch_subtraction_handles_sparse_and_dense_pairs() {
        let mut dense = OrdinalSet::from_ordinals(0..20, 128);
        let sparse = OrdinalSet::from_ordinals([1, 3, 5], 128);
        dense.subtract(&sparse, 128);
        assert_eq!(dense.len(), 17);
        assert!(!dense.contains(3));

        let mut sparse = OrdinalSet::from_ordinals([2, 40, 90], 1_000);
        let dense_removed = OrdinalSet::from_ordinals(0..50, 128);
        sparse.subtract(&dense_removed, 1_000);
        assert_eq!(sparse.iter().collect::<Vec<_>>(), vec![90]);
    }

    #[test]
    fn dense_bitmap_cached_cardinality_matches_iteration() {
        let mut bitmap = DenseBitmap::default();
        let mut expected = BTreeSet::new();
        let mut random = 0xD3A5_EB17_u64;
        for _ in 0..20_000 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let ordinal = (random % 4_096) as u32;
            let value = random & (1 << 12) != 0;
            assert_eq!(bitmap.set(ordinal, value), {
                if value {
                    expected.insert(ordinal)
                } else {
                    expected.remove(&ordinal)
                }
            });
            assert_eq!(bitmap.count(), expected.len());
            assert_eq!(
                bitmap.iter().collect::<Vec<_>>(),
                expected.iter().copied().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn ordinal_set_random_bulk_union_and_difference_match_reference_set() {
        let universe = 4_096;
        let mut random = 0xB01D_5E7_u64;
        let mut actual = OrdinalSet::default();
        let mut expected = BTreeSet::new();
        for _ in 0..250 {
            let mut values = BTreeSet::new();
            for _ in 0..128 {
                random = random
                    .wrapping_mul(2_862_933_555_777_941_757)
                    .wrapping_add(3_037_000_493);
                values.insert((random % universe as u64) as u32);
            }
            let operand = OrdinalSet::from_ordinals(values.iter().copied(), universe);
            let mut changed = Vec::new();
            if random & 1 == 0 {
                actual.union_with(&operand, universe, |ordinal| changed.push(ordinal));
                let expected_changed = values
                    .iter()
                    .filter(|ordinal| !expected.contains(*ordinal))
                    .copied()
                    .collect::<Vec<_>>();
                expected.extend(values);
                assert_eq!(changed, expected_changed);
            } else {
                actual.subtract_with(&operand, universe, |ordinal| changed.push(ordinal));
                let expected_changed = values
                    .iter()
                    .filter(|ordinal| expected.contains(*ordinal))
                    .copied()
                    .collect::<Vec<_>>();
                for ordinal in values {
                    expected.remove(&ordinal);
                }
                assert_eq!(changed, expected_changed);
            }
            assert_eq!(
                actual.iter().collect::<Vec<_>>(),
                expected.iter().copied().collect::<Vec<_>>()
            );
            assert_eq!(actual.len(), expected.len());
        }
    }

    #[test]
    fn bulk_membership_edits_keep_counts_and_has_any_indexes_consistent() {
        let universe = 256;
        let targets = OrdinalSet::from_ordinals(0..128, universe);
        let overlap = OrdinalSet::from_ordinals(64..192, universe);
        let mut indexes = QueryIndexes::default();
        indexes.edit_tags(
            &targets,
            &BTreeSet::from(["first".to_owned()]),
            &BTreeSet::new(),
            universe,
        );
        indexes.edit_tags(
            &overlap,
            &BTreeSet::from(["second".to_owned()]),
            &BTreeSet::new(),
            universe,
        );
        assert_eq!(indexes.tags["first"].len(), 128);
        assert_eq!(indexes.tags["second"].len(), 128);
        assert_eq!(indexes.tag_membership_count[63], 1);
        assert_eq!(indexes.tag_membership_count[64], 2);
        assert!(indexes.has_any_tag.contains(191));

        indexes.edit_tags(
            &targets,
            &BTreeSet::new(),
            &BTreeSet::from(["first".to_owned(), "second".to_owned()]),
            universe,
        );
        assert!(!indexes.has_any_tag.contains(63));
        assert!(!indexes.has_any_tag.contains(64));
        assert!(indexes.has_any_tag.contains(128));
        assert_eq!(indexes.tag_membership_count[64], 0);
        assert_eq!(indexes.tag_membership_count[128], 1);
    }

    #[test]
    fn album_override_refreshes_size_range_and_explicit_cover_thumbhash() {
        let album_id = ArrayString::<64>::from("album").unwrap();
        let media_id = ArrayString::<64>::from("media").unwrap();
        let mut media_metadata = crate::public::structure::image::ImageMetadata::new(
            media_id,
            10,
            20,
            30,
            "jpg".to_string(),
        );
        media_metadata.albums.insert(album_id);
        let mut media_object =
            crate::public::structure::object::ObjectSchema::new(media_id, ObjectType::Image);
        media_object.thumbhash = Some(vec![1]);
        let media = AbstractData::Image(crate::public::structure::image::ImageCombined {
            object: media_object,
            metadata: media_metadata,
        });

        let mut album_metadata = crate::public::structure::album::metadata::AlbumMetadata {
            id: album_id,
            cover: Some(media_id),
            ..crate::public::structure::album::metadata::AlbumMetadata::default()
        };
        album_metadata.item_count = 1;
        album_metadata.item_size = 10;
        let mut album_object =
            crate::public::structure::object::ObjectSchema::new(album_id, ObjectType::Album);
        album_object.thumbhash = Some(vec![1]);
        let album = AbstractData::Album(AlbumCombined {
            object: album_object,
            metadata: album_metadata,
        });

        let state = TreeState::from_records([media.clone(), album]);
        let slot_ref = state.find(media_id.as_str()).unwrap();
        let mut updated = media;
        updated.set_size(99);
        updated.set_thumbhash(vec![9]);
        updated.set_cache_version(7);
        let changed_at = crate::public::structure::object::next_mutation_timestamp();
        let aggregate = state
            .album_aggregate_with_override(album_id, slot_ref, &updated, changed_at)
            .unwrap();

        assert_eq!(aggregate.metadata.item_count, 1);
        assert_eq!(aggregate.metadata.item_size, 99);
        assert_eq!(aggregate.metadata.cover, Some(media_id));
        assert_eq!(aggregate.object.thumbhash, Some(vec![9]));
        assert_eq!(aggregate.object.cache_version, 7);
        assert_eq!(aggregate.object.update_at, changed_at);
        assert_eq!(aggregate.metadata.last_modified_time, changed_at);
        assert!(aggregate.metadata.start_time.is_some());
        assert_eq!(aggregate.metadata.start_time, aggregate.metadata.end_time);
    }

    #[test]
    fn album_override_accounts_for_direct_membership_add_and_remove() {
        let album_id = ArrayString::<64>::from("album-membership").unwrap();
        let media_id = ArrayString::<64>::from("media-membership").unwrap();
        let media = AbstractData::Image(crate::public::structure::image::ImageCombined {
            object: crate::public::structure::object::ObjectSchema::new(
                media_id,
                ObjectType::Image,
            ),
            metadata: crate::public::structure::image::ImageMetadata::new(
                media_id,
                10,
                20,
                30,
                "jpg".to_owned(),
            ),
        });
        let album = AbstractData::Album(AlbumCombined {
            object: crate::public::structure::object::ObjectSchema::new(
                album_id,
                ObjectType::Album,
            ),
            metadata: crate::public::structure::album::metadata::AlbumMetadata {
                id: album_id,
                ..crate::public::structure::album::metadata::AlbumMetadata::default()
            },
        });

        let state_without_member = TreeState::from_records([media.clone(), album.clone()]);
        let slot_ref = state_without_member.find(media_id.as_str()).unwrap();
        let mut added = media;
        added.albums_mut().unwrap().insert(album_id);
        added.set_thumbhash(vec![5]);
        added.set_cache_version(5);
        let added_aggregate = state_without_member
            .album_aggregate_with_override(
                album_id,
                slot_ref,
                &added,
                crate::public::structure::object::next_mutation_timestamp(),
            )
            .unwrap();
        assert_eq!(added_aggregate.metadata.item_count, 1);
        assert_eq!(added_aggregate.metadata.cover, Some(media_id));
        assert_eq!(added_aggregate.object.cache_version, 5);

        let state_with_member = TreeState::from_records([added.clone(), album]);
        let slot_ref = state_with_member.find(media_id.as_str()).unwrap();
        let mut removed = added;
        removed.albums_mut().unwrap().remove(&album_id);
        let removed_aggregate = state_with_member
            .album_aggregate_with_override(
                album_id,
                slot_ref,
                &removed,
                crate::public::structure::object::next_mutation_timestamp(),
            )
            .unwrap();
        assert_eq!(removed_aggregate.metadata.item_count, 0);
        assert_eq!(removed_aggregate.metadata.cover, None);
        assert_eq!(removed_aggregate.object.cache_version, 0);
    }

    #[test]
    fn cached_album_object_edits_publish_the_same_mutation_timestamp() {
        let album_id = ArrayString::<64>::from("cached-album-object").unwrap();
        let album = Album::new(album_id, None).into_abstract_data();
        let mut state = TreeState::from_records([album]);
        let album_slot = state.find(album_id.as_str()).unwrap();
        let targets = TargetSet::from_slot_refs([album_slot], state.arena.capacity());
        let changed_at = crate::public::structure::object::next_mutation_timestamp();

        state.edit_cached_album_objects(&targets, changed_at, |object| {
            object.tags.insert("cached".to_owned());
            object.description = Some("updated".to_owned());
        });

        let album = state.albums.get(&album_id).unwrap();
        assert_eq!(album.object.update_at, changed_at);
        assert!(album.object.tags.contains("cached"));
        assert_eq!(album.object.description.as_deref(), Some("updated"));
    }

    #[test]
    fn target_set_rejects_a_reused_generation_without_storing_common_generations() {
        let original = SlotRef::new(7, 1);
        let reused = SlotRef::new(7, 2);
        let common = TargetSet::from_slot_refs([original], 64);
        let override_set = TargetSet::from_slot_refs([reused], 64);
        assert!(common.contains(original));
        assert!(!common.contains(reused));
        assert!(override_set.contains(reused));
        assert!(!override_set.contains(original));
        assert_eq!(common.generation_overrides.len(), 0);
        assert_eq!(override_set.generation_overrides, vec![(7, 2)]);
    }

    #[test]
    fn target_set_bulk_subtraction_preserves_generation_identity() {
        let mut targets = TargetSet::from_slot_refs(
            [SlotRef::new(1, 1), SlotRef::new(7, 2), SlotRef::new(9, 1)],
            64,
        );
        targets.subtract(&TargetSet::from_slot_refs(
            [SlotRef::new(1, 1), SlotRef::new(7, 1)],
            64,
        ));
        assert!(!targets.contains(SlotRef::new(1, 1)));
        assert!(targets.contains(SlotRef::new(7, 2)));
        assert!(targets.contains(SlotRef::new(9, 1)));
    }

    #[test]
    fn target_set_union_merges_ordinals_and_keeps_latest_generation() {
        let mut targets = TargetSet::from_slot_refs([SlotRef::new(1, 1), SlotRef::new(7, 2)], 128);
        targets.union(
            &TargetSet::from_slot_refs(
                [SlotRef::new(2, 1), SlotRef::new(7, 3), SlotRef::new(90, 1)],
                128,
            ),
            128,
        );
        assert_eq!(targets.len(), 4);
        assert!(targets.contains(SlotRef::new(1, 1)));
        assert!(targets.contains(SlotRef::new(2, 1)));
        assert!(targets.contains(SlotRef::new(7, 3)));
        assert!(!targets.contains(SlotRef::new(7, 2)));
        assert!(targets.contains(SlotRef::new(90, 1)));
    }

    #[test]
    fn target_set_changed_bitmap_is_wordwise_and_preserves_generations() {
        let targets = TargetSet::from_unique_slot_refs(
            [SlotRef::new(1, 1), SlotRef::new(7, 2), SlotRef::new(65, 1)],
            128,
        );
        let mut current = DenseBitmap::default();
        current.set(1, true);
        current.set(65, true);

        let enabling = targets.changed_for_bitmap(&current, true, 128);
        assert_eq!(
            enabling.iter().collect::<Vec<_>>(),
            vec![SlotRef::new(7, 2)]
        );
        let disabling = targets.changed_for_bitmap(&current, false, 128);
        assert_eq!(
            disabling.iter().collect::<Vec<_>>(),
            vec![SlotRef::new(1, 1), SlotRef::new(65, 1)]
        );
    }

    #[test]
    fn dense_target_flag_round_trip_reports_every_changed_ordinal() {
        let universe = 1_024;
        let targets = TargetSet::from_unique_slot_refs(
            (0..938).map(|ordinal| SlotRef::new(ordinal, 1)),
            universe,
        );
        assert!(matches!(targets.ordinals(), OrdinalSet::Dense(_)));
        let mut indexes = QueryIndexes::default();

        let enabling = targets.changed_for_bitmap(&indexes.favorite, true, universe);
        assert_eq!(enabling.len(), targets.len());
        indexes.edit_flags(
            targets.ordinals(),
            FlagPatch {
                favorite: Some(true),
                ..FlagPatch::default()
            },
        );
        assert_eq!(indexes.favorite.count(), targets.len());

        let disabling = targets.changed_for_bitmap(&indexes.favorite, false, universe);
        assert_eq!(disabling.len(), targets.len());
        indexes.edit_flags(
            targets.ordinals(),
            FlagPatch {
                favorite: Some(false),
                ..FlagPatch::default()
            },
        );
        assert_eq!(indexes.favorite.count(), 0);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn structural_epoch_changes_only_for_structural_tree_mutations() {
        let mut records = (0..4)
            .map(|index| AbstractData::generate_performance_data(index, 19))
            .collect::<Vec<_>>();
        let mut state = TreeState::from_records(records.iter().cloned());
        let initial_epoch = state.structural_epoch();
        let targets = OrdinalSet::from_ordinals([0, 1], state.arena.capacity());
        state.query.edit_flags(
            &targets,
            FlagPatch {
                favorite: Some(true),
                ..FlagPatch::default()
            },
        );
        assert_eq!(state.structural_epoch(), initial_epoch);

        let rekeyed = state.find(records[0].hash().as_str()).unwrap();
        records[0].alias_mut().unwrap()[0].modified += 1;
        state.replace_static(rekeyed, &records[0]).unwrap();
        let rekeyed_epoch = state.structural_epoch();
        assert_ne!(rekeyed_epoch, initial_epoch);

        let inserted = state.insert(&AbstractData::generate_performance_data(100, 19));
        let inserted_epoch = state.structural_epoch();
        assert_ne!(inserted_epoch, rekeyed_epoch);
        state.remove_targets(&TargetSet::from_slot_refs(
            [inserted],
            state.arena.capacity(),
        ));
        assert_ne!(state.structural_epoch(), inserted_epoch);
    }

    #[test]
    fn id_index_verifies_full_id_inside_collision_bucket() {
        let mut arena = RecordArena::default();
        let first = arena.allocate(cache_record("first", 1));
        let second = arena.allocate(cache_record("second", 1));
        let mut index = IdIndex::default();
        let fingerprint = IdIndex::fingerprint("second");
        index.primary.insert(fingerprint, first);
        index.collisions.insert(fingerprint, vec![first, second]);
        assert_eq!(index.find("second", &arena), Some(second));
        assert_eq!(index.find("missing", &arena), None);

        index.remove("second", second);
        assert!(!index.collisions.contains_key(&fingerprint));
        assert_eq!(index.primary.get(&fingerprint), Some(&first));
        assert_eq!(index.find("second", &arena), None);
    }

    #[test]
    fn id_index_insert_find_and_remove_use_the_primary_table() {
        let mut arena = RecordArena::default();
        let slot = arena.allocate(cache_record("primary", 1));
        let mut index = IdIndex::default();
        index.insert("primary", slot);
        assert_eq!(index.find("primary", &arena), Some(slot));
        assert!(index.collisions.is_empty());
        index.remove("primary", slot);
        assert_eq!(index.find("primary", &arena), None);
        assert!(index.primary.is_empty());
    }

    #[test]
    fn id_index_rejects_a_stale_slot_generation() {
        let mut arena = RecordArena::default();
        let stale = arena.allocate(cache_record("stale", 1));
        let mut index = IdIndex::default();
        index.insert("stale", stale);
        assert!(arena.remove(stale).is_some());
        let replacement = arena.allocate(cache_record("replacement", 1));
        assert_eq!(stale.index(), replacement.index());
        assert_ne!(stale.generation(), replacement.generation());
        assert_eq!(index.find("stale", &arena), None);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn tree_memory_usage_increases_with_owned_records() {
        let empty = TreeState::default().memory_usage();
        let mut state = TreeState::from_records(
            (0..64).map(|index| AbstractData::generate_performance_data(index, 77)),
        );
        let populated = state.memory_usage();
        assert!(populated.total_bytes() > empty.total_bytes());
        assert!(populated.arena_inline_bytes > empty.arena_inline_bytes);
        assert!(populated.record_dynamic_bytes > empty.record_dynamic_bytes);
        assert!(populated.id_index_bytes > empty.id_index_bytes);
        assert!(populated.order_index_bytes > empty.order_index_bytes);
        let exact_dynamic_bytes = state
            .arena
            .slots
            .iter()
            .filter_map(|slot| slot.value.as_ref())
            .map(CacheRecord::estimated_dynamic_bytes)
            .sum::<usize>();
        assert_eq!(populated.record_dynamic_bytes, exact_dynamic_bytes);

        state = TreeState::default();
        assert_eq!(state.memory_usage(), empty);
    }

    #[test]
    fn equal_timestamps_are_ordered_by_object_id() {
        let mut arena = RecordArena::default();
        let second = arena.allocate(cache_record("b", 10));
        let first = arena.allocate(cache_record("a", 10));
        let mut order = vec![second];
        sort_and_merge(&arena, &mut order, vec![first]);
        assert_eq!(order, vec![first, second]);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn compiled_parallel_query_matches_sequential_order() {
        use rayon::prelude::*;

        let records = (0..10_000)
            .map(|index| AbstractData::generate_performance_data(index, 77))
            .collect::<Vec<_>>();
        let state = TreeState::from_records(records);
        let expression = Expression::And(vec![
            Expression::Or(vec![
                Expression::Favorite(true),
                Expression::Ext("JpG".to_owned()),
                Expression::Any("camera".to_owned()),
            ]),
            Expression::Not(Box::new(Expression::Trashed(true))),
        ]);
        let compiled = state.compile_expression(&expression, None);
        let sequential = state
            .order
            .iter()
            .filter(|slot_ref| {
                let record = state.get(**slot_ref).unwrap();
                compiled.matches(record, slot_ref.index())
            })
            .map(|slot_ref| slot_ref.raw())
            .collect::<Vec<_>>();
        let parallel = state
            .order
            .par_iter()
            .filter(|slot_ref| {
                let record = state.get(**slot_ref).unwrap();
                compiled.matches(record, slot_ref.index())
            })
            .map(|slot_ref| slot_ref.raw())
            .collect::<Vec<_>>();
        assert_eq!(parallel, sequential);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn incremental_order_matches_full_sort() {
        let records = (0..256)
            .map(|index| AbstractData::generate_performance_data(index, 42))
            .collect::<Vec<_>>();
        let mut state = TreeState::from_records(records[..128].iter().cloned());
        for record in &records[128..] {
            state.insert(record);
        }
        let removed = state
            .order
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, slot_ref)| (index % 5 == 0).then_some(slot_ref))
            .collect::<Vec<_>>();
        for slot_ref in removed {
            state.remove(slot_ref);
        }
        for index in 300..360 {
            state.insert(&AbstractData::generate_performance_data(index, 42));
        }
        let mut expected = state.order.as_ref().clone();
        expected.sort_unstable_by(|left, right| compare_slots(&state.arena, *left, *right));
        assert_eq!(state.order.as_ref(), &expected);
        for pair in state.order.windows(2) {
            assert_ne!(
                compare_slots(&state.arena, pair[0], pair[1]),
                Ordering::Greater
            );
        }
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn randomized_batch_structure_matches_full_sort() {
        let initial = (0..64)
            .map(|index| AbstractData::generate_performance_data(index, 91))
            .collect::<Vec<_>>();
        let mut records = initial
            .iter()
            .cloned()
            .map(|record| (record.hash(), record))
            .collect::<HashMap<_, _>>();
        let mut state = TreeState::from_records(initial);
        let mut random = 0xA11C_E5EED_u64;
        let mut next_index = 64_u64;

        for step in 0..500 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            match random % 4 {
                0 => {
                    let record = AbstractData::generate_performance_data(next_index, 91);
                    next_index += 1;
                    records.insert(record.hash(), record.clone());
                    state.apply_batch(&[record], &HashSet::new());
                }
                1 if !state.order.is_empty() => {
                    let position = random as usize % state.order.len();
                    let slot_ref = state.order[position];
                    let id = state.get(slot_ref).unwrap().id;
                    let targets = TargetSet::from_slot_refs([slot_ref], state.arena.capacity());
                    state.remove_targets(&targets);
                    records.remove(&id);
                }
                2 if !state.order.is_empty() => {
                    let position = random as usize % state.order.len();
                    let id = state.get(state.order[position]).unwrap().id;
                    let record = records.get_mut(&id).unwrap();
                    if let Some(alias) = record.alias_mut().and_then(|aliases| aliases.first_mut())
                    {
                        alias.modified = alias.modified.saturating_add(10_000 + step);
                        alias.scan_time = alias.modified;
                    }
                    state.apply_batch(&[record.clone()], &HashSet::new());
                }
                _ => {
                    let additions = (0..3)
                        .map(|_| {
                            let record = AbstractData::generate_performance_data(next_index, 91);
                            next_index += 1;
                            records.insert(record.hash(), record.clone());
                            record
                        })
                        .collect::<Vec<_>>();
                    let removals = state
                        .order
                        .iter()
                        .take(2)
                        .filter_map(|slot_ref| state.get(*slot_ref).map(|record| record.id))
                        .collect::<HashSet<_>>();
                    for id in &removals {
                        records.remove(id);
                    }
                    state.apply_batch(&additions, &removals);
                }
            }

            let mut expected = state.order.as_ref().clone();
            expected.sort_unstable_by(|left, right| compare_slots(&state.arena, *left, *right));
            assert_eq!(state.order.as_ref(), &expected);
            assert_eq!(state.order.len(), state.len());
            assert_eq!(
                state
                    .order
                    .iter()
                    .map(|slot_ref| state.get(*slot_ref).unwrap().id)
                    .collect::<HashSet<_>>(),
                records.keys().copied().collect::<HashSet<_>>()
            );
        }
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn album_membership_and_trash_aggregates_update_incrementally() {
        let album_id = ArrayString::<64>::from("aggregate-album").unwrap();
        let mut first = AbstractData::generate_performance_data(700, 19);
        let mut second = AbstractData::generate_performance_data(701, 19);
        for record in [&mut first, &mut second] {
            record.set_trashed(false);
            record.set_archived(false);
        }
        let first_id = first.hash();
        let second_id = second.hash();
        let album = Album::new(album_id, Some("Aggregate".to_owned())).into_abstract_data();
        let mut state = TreeState::from_records([first, second, album]);
        let first_slot = state.find(first_id.as_str()).unwrap();
        let second_slot = state.find(second_id.as_str()).unwrap();
        let both = OrdinalSet::from_ordinals(
            [first_slot.index(), second_slot.index()],
            state.arena.capacity(),
        );

        let patches =
            state.edit_album_memberships(&both, &BTreeSet::from([album_id]), &BTreeSet::new(), 100);
        assert_eq!(patches[0].metadata.item_count, 2);
        assert_eq!(state.query.albums[&album_id].len(), 2);

        let first_only = OrdinalSet::from_ordinals([first_slot.index()], state.arena.capacity());
        let patches = state.edit_flags_and_refresh(
            &first_only,
            FlagPatch {
                trashed: Some(true),
                ..FlagPatch::default()
            },
            101,
        );
        assert_eq!(patches[0].metadata.item_count, 1);

        let patches = state.edit_album_memberships(
            &OrdinalSet::from_ordinals([second_slot.index()], state.arena.capacity()),
            &BTreeSet::new(),
            &BTreeSet::from([album_id]),
            102,
        );
        assert_eq!(patches[0].metadata.item_count, 0);
        assert_eq!(patches[0].metadata.cover, None);
    }
}
