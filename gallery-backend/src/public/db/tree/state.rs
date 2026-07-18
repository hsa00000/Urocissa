use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use arrayvec::ArrayString;

use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::AlbumCombined;
use crate::public::structure::expression::{AlbumFilterValue, Expression, FilterValue};
use crate::public::structure::object::ObjectType;
use crate::public::structure::response::reduced_data::ReducedData;

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

#[derive(Debug, Clone)]
enum IdBucket {
    One(SlotRef),
    Many(Vec<SlotRef>),
}

#[derive(Debug, Default)]
pub struct IdIndex {
    buckets: HashMap<u64, IdBucket>,
}

impl IdIndex {
    fn fingerprint(id: &str) -> u64 {
        let bytes = blake3::hash(id.as_bytes());
        u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("eight-byte slice"))
    }

    pub fn insert(&mut self, id: &str, slot_ref: SlotRef) {
        use std::collections::hash_map::Entry;
        match self.buckets.entry(Self::fingerprint(id)) {
            Entry::Vacant(entry) => {
                entry.insert(IdBucket::One(slot_ref));
            }
            Entry::Occupied(mut entry) => match entry.get_mut() {
                IdBucket::One(existing) => {
                    *entry.get_mut() = IdBucket::Many(vec![*existing, slot_ref]);
                }
                IdBucket::Many(bucket) => bucket.push(slot_ref),
            },
        }
    }

    pub fn find(&self, id: &str, arena: &RecordArena<CacheRecord>) -> Option<SlotRef> {
        let matches = |slot_ref: &SlotRef| {
            arena
                .get(*slot_ref)
                .is_some_and(|record| record.id.as_str() == id)
        };
        match self.buckets.get(&Self::fingerprint(id))? {
            IdBucket::One(slot_ref) => matches(slot_ref).then_some(*slot_ref),
            IdBucket::Many(bucket) => bucket.iter().find(|slot_ref| matches(slot_ref)).copied(),
        }
    }

    pub fn remove(&mut self, id: &str, slot_ref: SlotRef) {
        let fingerprint = Self::fingerprint(id);
        let Some(bucket) = self.buckets.get_mut(&fingerprint) else {
            return;
        };
        match bucket {
            IdBucket::One(existing) if *existing == slot_ref => {
                self.buckets.remove(&fingerprint);
            }
            IdBucket::Many(items) => {
                items.retain(|item| *item != slot_ref);
                match items.as_slice() {
                    [] => {
                        self.buckets.remove(&fingerprint);
                    }
                    [remaining] => {
                        let remaining = *remaining;
                        self.buckets.insert(fingerprint, IdBucket::One(remaining));
                    }
                    _ => {}
                }
            }
            IdBucket::One(_) => {}
        }
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
    pub ext: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub path_aliases: Vec<String>,
}

impl CacheRecord {
    pub fn from_abstract_data(data: &AbstractData, timestamp: i64) -> Self {
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DenseBitmap(Vec<u64>);

impl DenseBitmap {
    pub fn contains(&self, ordinal: u32) -> bool {
        let word = ordinal as usize / 64;
        let bit = ordinal % 64;
        self.0
            .get(word)
            .is_some_and(|value| value & (1_u64 << bit) != 0)
    }

    pub fn set(&mut self, ordinal: u32, value: bool) -> bool {
        let word = ordinal as usize / 64;
        let bit = ordinal % 64;
        if value && word >= self.0.len() {
            self.0.resize(word + 1, 0);
        }
        let Some(entry) = self.0.get_mut(word) else {
            return false;
        };
        let before = *entry & (1_u64 << bit) != 0;
        if value {
            *entry |= 1_u64 << bit;
        } else {
            *entry &= !(1_u64 << bit);
        }
        before != value
    }

    pub fn count(&self) -> usize {
        self.0.iter().map(|word| word.count_ones() as usize).sum()
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().enumerate().flat_map(|(word_index, word)| {
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
            Self::Dense(bitmap) => bitmap.0.capacity() * std::mem::size_of::<u64>(),
        }
    }

    pub fn subtract(&mut self, removed: &Self, universe: usize) {
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
                for (word, removed_word) in bitmap.0.iter_mut().zip(&removed.0) {
                    *word &= !removed_word;
                }
            }
        }
        self.rebalance(universe);
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
        for slot_ref in removed.iter() {
            if !self.contains(slot_ref) {
                continue;
            }
            match &mut self.ordinals {
                OrdinalSet::Sparse(items) => {
                    if let Ok(index) = items.binary_search(&slot_ref.index()) {
                        items.remove(index);
                    }
                }
                OrdinalSet::Dense(bitmap) => {
                    bitmap.set(slot_ref.index(), false);
                }
            }
        }
        self.generation_overrides
            .retain(|(ordinal, generation)| !removed.contains(SlotRef::new(*ordinal, *generation)));
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
        for ordinal in targets.iter() {
            if let Some(value) = patch.favorite {
                self.favorite.set(ordinal, value);
            }
            if let Some(value) = patch.archived {
                self.archived.set(ordinal, value);
            }
            if let Some(value) = patch.trashed {
                self.trashed.set(ordinal, value);
            }
        }
    }

    pub fn edit_tags(
        &mut self,
        targets: &OrdinalSet,
        add: &BTreeSet<String>,
        remove: &BTreeSet<String>,
        universe: usize,
    ) {
        for ordinal in targets.iter() {
            self.ensure_ordinal(ordinal);
        }
        for tag in add {
            let members = self.tags.entry(tag.clone()).or_default();
            for ordinal in targets.iter() {
                if members.insert(ordinal, universe) {
                    self.tag_membership_count[ordinal as usize] =
                        self.tag_membership_count[ordinal as usize].saturating_add(1);
                }
            }
        }
        for tag in remove {
            if let Some(members) = self.tags.get_mut(tag) {
                for ordinal in targets.iter() {
                    if members.remove(ordinal, universe) {
                        self.tag_membership_count[ordinal as usize] =
                            self.tag_membership_count[ordinal as usize].saturating_sub(1);
                    }
                }
            }
        }
        self.tags.retain(|_, members| !members.is_empty());
        for ordinal in targets.iter() {
            self.has_any_tag
                .set(ordinal, self.tag_membership_count[ordinal as usize] > 0);
        }
    }

    pub fn edit_albums(
        &mut self,
        targets: &OrdinalSet,
        add: &BTreeSet<ArrayString<64>>,
        remove: &BTreeSet<ArrayString<64>>,
        universe: usize,
    ) {
        for ordinal in targets.iter() {
            self.ensure_ordinal(ordinal);
        }
        for album in add {
            let members = self.albums.entry(*album).or_default();
            for ordinal in targets.iter() {
                if members.insert(ordinal, universe) {
                    self.album_membership_count[ordinal as usize] =
                        self.album_membership_count[ordinal as usize].saturating_add(1);
                }
            }
        }
        for album in remove {
            if let Some(members) = self.albums.get_mut(album) {
                for ordinal in targets.iter() {
                    if members.remove(ordinal, universe) {
                        self.album_membership_count[ordinal as usize] =
                            self.album_membership_count[ordinal as usize].saturating_sub(1);
                    }
                }
            }
        }
        self.albums.retain(|_, members| !members.is_empty());
        for ordinal in targets.iter() {
            self.has_any_album
                .set(ordinal, self.album_membership_count[ordinal as usize] > 0);
        }
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

#[derive(Debug, Default)]
pub struct TreeState {
    pub arena: RecordArena<CacheRecord>,
    pub id_index: IdIndex,
    pub order: Arc<Vec<SlotRef>>,
    pub query: QueryIndexes,
    pub albums: HashMap<ArrayString<64>, AlbumCombined>,
}

impl TreeState {
    pub fn from_records(records: impl IntoIterator<Item = AbstractData>) -> Self {
        let mut state = Self::default();
        for data in records {
            let timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
            let record = CacheRecord::from_abstract_data(&data, timestamp);
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
        TargetSet::from_slot_refs(
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
        slot_ref
    }

    pub fn remove(&mut self, slot_ref: SlotRef) -> Option<CacheRecord> {
        let id = self.arena.get(slot_ref)?.id;
        self.id_index.remove(id.as_str(), slot_ref);
        self.query
            .remove_record(slot_ref.index(), self.arena.capacity());
        self.albums.remove(&id);
        Arc::make_mut(&mut self.order).retain(|candidate| *candidate != slot_ref);
        self.arena.remove(slot_ref)
    }

    pub fn remove_targets(&mut self, targets: &TargetSet) {
        let universe = self.arena.capacity();
        self.query.remove_targets(targets.ordinals(), universe);
        for slot_ref in targets.iter() {
            let Some(id) = self.arena.get(slot_ref).map(|record| record.id) else {
                continue;
            };
            self.id_index.remove(id.as_str(), slot_ref);
            self.albums.remove(&id);
            self.arena.remove(slot_ref);
        }
        let next_order = self
            .order
            .iter()
            .copied()
            .filter(|slot_ref| !targets.contains(*slot_ref))
            .collect();
        self.order = Arc::new(next_order);
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
            self.arena.remove(*slot_ref);
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
                self.query
                    .insert_record(slot_ref.index(), data, final_universe);
                if let AbstractData::Album(album) = data {
                    self.albums.insert(album.object.id, album.clone());
                }
                if let Some(record) = self.arena.get_mut(slot_ref) {
                    *record = next_record;
                }
                if old_timestamp != timestamp {
                    rekeyed.insert(slot_ref);
                    additions.push(slot_ref);
                }
            } else {
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
        matches_expression(
            record,
            slot_ref.index(),
            &self.query,
            expression,
            hidden_metadata_album,
        )
    }

    pub fn edit_album_memberships(
        &mut self,
        targets: &OrdinalSet,
        add: &BTreeSet<ArrayString<64>>,
        remove: &BTreeSet<ArrayString<64>>,
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
            let mut cover_candidate = None::<(i64, ArrayString<64>, Option<Vec<u8>>)>;
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
                    && cover_candidate.as_ref().is_none_or(|(timestamp, id, _)| {
                        record.timestamp > *timestamp
                            || (record.timestamp == *timestamp && record.id < *id)
                    })
                {
                    cover_candidate = Some((record.timestamp, record.id, record.thumbhash.clone()));
                }
            }
            if album.metadata.cover.is_none()
                && let Some((_, id, thumbhash)) = cover_candidate
            {
                album.metadata.cover = Some(id);
                album.object.thumbhash = thumbhash;
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
                if let Some(album) = self.refresh_album_aggregate(album_id) {
                    patches.push(album);
                }
            } else {
                album.metadata.last_modified_time = chrono::Utc::now().timestamp_millis();
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
            let mut cover_candidate = None::<(i64, ArrayString<64>, Option<Vec<u8>>)>;
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
                        && cover_candidate.as_ref().is_none_or(|(timestamp, id, _)| {
                            record.timestamp > *timestamp
                                || (record.timestamp == *timestamp && record.id < *id)
                        })
                    {
                        cover_candidate =
                            Some((record.timestamp, record.id, record.thumbhash.clone()));
                    }
                }
            }
            if !trashed
                && album.metadata.cover.is_none()
                && let Some((_, id, thumbhash)) = cover_candidate
            {
                album.metadata.cover = Some(id);
                album.object.thumbhash = thumbhash;
            }
        }

        self.query.edit_flags(targets, patch);
        let mut patches = Vec::with_capacity(updated.len());
        for album_id in affected {
            let Some((mut album, rebuild)) = updated.remove(&album_id) else {
                continue;
            };
            if rebuild {
                if let Some(album) = self.refresh_album_aggregate(album_id) {
                    patches.push(album);
                }
            } else {
                album.metadata.last_modified_time = chrono::Utc::now().timestamp_millis();
                self.albums.insert(album_id, album.clone());
                patches.push(album);
            }
        }
        patches
    }

    pub fn refresh_album_aggregate(&mut self, album_id: ArrayString<64>) -> Option<AlbumCombined> {
        let album = self.album_aggregate_excluding(album_id, &TargetSet::default())?;
        self.albums.insert(album_id, album.clone());
        Some(album)
    }

    pub fn album_aggregate_excluding(
        &self,
        album_id: ArrayString<64>,
        excluded: &TargetSet,
    ) -> Option<AlbumCombined> {
        let mut album = self.albums.get(&album_id)?.clone();
        let current_cover = album.metadata.cover;
        let mut item_count = 0_usize;
        let mut item_size = 0_u64;
        let mut start_time = None::<i64>;
        let mut end_time = None::<i64>;
        let mut cover_is_member = false;
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
            cover_is_member |= current_cover == Some(record.id);
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
        album.metadata.last_modified_time = chrono::Utc::now().timestamp_millis();
        if item_count == 0 {
            album.metadata.cover = None;
            album.object.thumbhash = None;
        } else if !cover_is_member && let Some(record) = newest {
            album.metadata.cover = Some(record.id);
            album.object.thumbhash.clone_from(&record.thumbhash);
        }
        Some(album)
    }
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

fn value_matches(value: Option<&str>, filter: &FilterValue) -> bool {
    match filter {
        FilterValue::Value(needle) => value.is_some_and(|value| {
            value
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        }),
        FilterValue::Exists(exists) => value.is_some() == *exists,
    }
}

fn matches_expression(
    record: &CacheRecord,
    ordinal: u32,
    indexes: &QueryIndexes,
    expression: &Expression,
    hidden_metadata_album: Option<ArrayString<64>>,
) -> bool {
    match expression {
        Expression::Or(expressions) => expressions.iter().any(|expression| {
            matches_expression(record, ordinal, indexes, expression, hidden_metadata_album)
        }),
        Expression::And(expressions) => expressions.iter().all(|expression| {
            matches_expression(record, ordinal, indexes, expression, hidden_metadata_album)
        }),
        Expression::Not(expression) => {
            !matches_expression(record, ordinal, indexes, expression, hidden_metadata_album)
        }
        Expression::Tag(filter) => {
            if hidden_metadata_album.is_some() {
                return false;
            }
            match filter {
                FilterValue::Value(tag) => indexes
                    .tags
                    .get(tag)
                    .is_some_and(|members| members.contains(ordinal)),
                FilterValue::Exists(exists) => indexes.has_any_tag.contains(ordinal) == *exists,
            }
        }
        Expression::Favorite(value) => indexes.favorite.contains(ordinal) == *value,
        Expression::Archived(value) => indexes.archived.contains(ordinal) == *value,
        Expression::Trashed(value) => indexes.trashed.contains(ordinal) == *value,
        Expression::ExtType(ext_type) => match record.object_type {
            ObjectType::Image => ext_type.contains("image"),
            ObjectType::Video => ext_type.contains("video"),
            ObjectType::Album => hidden_metadata_album.is_none() && ext_type.contains("album"),
        },
        Expression::Ext(ext) => record
            .ext
            .to_ascii_lowercase()
            .contains(&ext.to_ascii_lowercase()),
        Expression::Model(filter) => value_matches(record.model.as_deref(), filter),
        Expression::Make(filter) => value_matches(record.make.as_deref(), filter),
        Expression::Path(path) => {
            hidden_metadata_album.is_none()
                && record.path_aliases.iter().any(|alias| {
                    alias
                        .to_ascii_lowercase()
                        .contains(&path.to_ascii_lowercase())
                })
        }
        Expression::Album(filter) => match filter {
            AlbumFilterValue::Value(album_id) => {
                if hidden_metadata_album.is_some_and(|allowed| allowed != *album_id) {
                    return false;
                }
                indexes
                    .albums
                    .get(album_id)
                    .is_some_and(|members| members.contains(ordinal))
            }
            AlbumFilterValue::Exists(exists) => {
                if hidden_metadata_album.is_some() {
                    record.object_type != ObjectType::Album && *exists
                } else {
                    indexes.has_any_album.contains(ordinal) == *exists
                }
            }
        },
        Expression::Any(value) => {
            let lower = value.to_ascii_lowercase();
            let static_match = record.id.as_str().to_ascii_lowercase().contains(&lower)
                || record.ext.to_ascii_lowercase().contains(&lower)
                || record
                    .make
                    .as_ref()
                    .is_some_and(|item| item.to_ascii_lowercase().contains(&lower))
                || record
                    .model
                    .as_ref()
                    .is_some_and(|item| item.to_ascii_lowercase().contains(&lower))
                || match record.object_type {
                    ObjectType::Image => "image".contains(&lower),
                    ObjectType::Video => "video".contains(&lower),
                    ObjectType::Album => {
                        hidden_metadata_album.is_none() && "album".contains(&lower)
                    }
                };
            if hidden_metadata_album.is_some() {
                static_match
            } else {
                static_match
                    || indexes
                        .tags
                        .get(value)
                        .is_some_and(|members| members.contains(ordinal))
                    || record
                        .path_aliases
                        .iter()
                        .any(|path| path.to_ascii_lowercase().contains(&lower))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache_record(id: &str, timestamp: i64) -> CacheRecord {
        CacheRecord {
            id: ArrayString::<64>::from(id).unwrap(),
            object_type: ObjectType::Image,
            timestamp,
            width: 1,
            height: 1,
            size: 1,
            thumbhash: None,
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
    fn id_index_verifies_full_id_inside_collision_bucket() {
        let mut arena = RecordArena::default();
        let first = arena.allocate(cache_record("first", 1));
        let second = arena.allocate(cache_record("second", 1));
        let mut index = IdIndex::default();
        index.buckets.insert(
            IdIndex::fingerprint("second"),
            IdBucket::Many(vec![first, second]),
        );
        assert_eq!(index.find("second", &arena), Some(second));
        assert_eq!(index.find("missing", &arena), None);
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
        use crate::public::structure::album::Album;

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
            state.edit_album_memberships(&both, &BTreeSet::from([album_id]), &BTreeSet::new());
        assert_eq!(patches[0].metadata.item_count, 2);
        assert_eq!(state.query.albums[&album_id].len(), 2);

        let first_only = OrdinalSet::from_ordinals([first_slot.index()], state.arena.capacity());
        let patches = state.edit_flags_and_refresh(
            &first_only,
            FlagPatch {
                trashed: Some(true),
                ..FlagPatch::default()
            },
        );
        assert_eq!(patches[0].metadata.item_count, 1);

        let patches = state.edit_album_memberships(
            &OrdinalSet::from_ordinals([second_slot.index()], state.arena.capacity()),
            &BTreeSet::new(),
            &BTreeSet::from([album_id]),
        );
        assert_eq!(patches[0].metadata.item_count, 0);
        assert_eq!(patches[0].metadata.cover, None);
    }
}
