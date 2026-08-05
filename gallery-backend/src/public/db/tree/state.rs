use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};
use std::iter::FusedIterator;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, LazyLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use arrayvec::ArrayString;

use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::album::AlbumCombined;
use crate::public::structure::expression::{AlbumFilterValue, Expression, FilterValue};
use crate::public::structure::object::{ObjectSchema, ObjectType};

static NEXT_TREE_EPOCH: LazyLock<AtomicU64> = LazyLock::new(|| {
    let time_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |duration| {
            u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
        });
    AtomicU64::new((time_seed ^ u64::from(std::process::id())).max(1))
});

fn next_tree_epoch() -> u64 {
    NEXT_TREE_EPOCH.fetch_add(1, AtomicOrdering::Relaxed)
}

#[allow(dead_code)]
fn exact_size_hint(iterator: &impl Iterator) -> usize {
    let (lower, upper) = iterator.size_hint();
    upper.filter(|upper| *upper == lower).unwrap_or(lower)
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

    /// Returns the packed low 32 bits written by [`Self::new`].
    #[allow(clippy::cast_possible_truncation)]
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

fn natural_vec_capacity<T>(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let minimum = match std::mem::size_of::<T>() {
        1 => 8,
        2..=1024 => 4,
        _ => 1,
    };
    len.checked_next_power_of_two().unwrap_or(len).max(minimum)
}

impl<T> RecordArena<T> {
    fn with_record_capacity(record_count: usize) -> Self {
        Self {
            slots: Vec::with_capacity(natural_vec_capacity::<ArenaSlot<T>>(record_count)),
            free: Vec::new(),
            len: 0,
        }
    }

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
}

#[derive(Debug, Default)]
struct U64IdentityHasher(u64);

impl Hasher for U64IdentityHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // HashMap's u64 key implementation calls `write_u64`. Keep a
        // deterministic fallback for completeness without weakening that
        // identity fast path.
        self.0 = bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        });
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

type FingerprintMap<V> = HashMap<u64, V, BuildHasherDefault<U64IdentityHasher>>;

#[derive(Debug, Default)]
pub struct IdIndex {
    primary: FingerprintMap<u32>,
    collisions: FingerprintMap<Vec<u32>>,
}

impl IdIndex {
    fn with_record_capacity(record_count: usize) -> Self {
        Self {
            primary: FingerprintMap::with_capacity_and_hasher(
                record_count,
                BuildHasherDefault::default(),
            ),
            collisions: FingerprintMap::default(),
        }
    }

    fn fingerprint(id: &str) -> u64 {
        if let Some(fingerprint) = canonical_hex_fingerprint(id) {
            return fingerprint;
        }
        Self::legacy_fingerprint(id)
    }

    fn legacy_fingerprint(id: &str) -> u64 {
        let bytes = blake3::hash(id.as_bytes());
        u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("eight-byte slice"))
    }

    pub fn insert(&mut self, id: &str, slot_ref: SlotRef) {
        use std::collections::hash_map::Entry;
        let fingerprint = Self::fingerprint(id);
        let ordinal = slot_ref.index();
        if let Some(bucket) = self.collisions.get_mut(&fingerprint) {
            if !bucket.contains(&ordinal) {
                bucket.push(ordinal);
            }
            return;
        }
        match self.primary.entry(fingerprint) {
            Entry::Vacant(entry) => {
                entry.insert(ordinal);
            }
            Entry::Occupied(entry) => {
                let existing = *entry.get();
                if existing != ordinal {
                    self.collisions.insert(fingerprint, vec![existing, ordinal]);
                }
            }
        }
    }

    pub fn find(&self, id: &str, arena: &RecordArena<CacheRecord>) -> Option<SlotRef> {
        let matching_slot = |ordinal: &u32| {
            let slot_ref = arena.slot_at_ordinal(*ordinal)?;
            arena
                .get(slot_ref)
                .is_some_and(|record| record.id.as_str() == id)
                .then_some(slot_ref)
        };
        let fingerprint = Self::fingerprint(id);
        if let Some(bucket) = self.collisions.get(&fingerprint) {
            return bucket.iter().find_map(matching_slot);
        }
        self.primary.get(&fingerprint).and_then(matching_slot)
    }

    pub fn remove(
        &mut self,
        id: &str,
        slot_ref: SlotRef,
        arena: &RecordArena<CacheRecord>,
    ) -> bool {
        if arena
            .get(slot_ref)
            .is_none_or(|record| record.id.as_str() != id)
        {
            return false;
        }
        let fingerprint = Self::fingerprint(id);
        let ordinal = slot_ref.index();
        if let Some(bucket) = self.collisions.get_mut(&fingerprint) {
            let previous_len = bucket.len();
            bucket.retain(|item| *item != ordinal);
            if bucket.len() == previous_len {
                return false;
            }
            let remaining = bucket.first().copied();
            if bucket.len() > 1 {
                if let Some(remaining) = remaining {
                    self.primary.insert(fingerprint, remaining);
                }
                return true;
            }
            self.collisions.remove(&fingerprint);
            if let Some(remaining) = remaining {
                self.primary.insert(fingerprint, remaining);
            } else {
                self.primary.remove(&fingerprint);
            }
            return true;
        }
        if self.primary.get(&fingerprint) == Some(&ordinal) {
            self.primary.remove(&fingerprint);
            return true;
        }
        false
    }

    #[cfg(feature = "performance-test")]
    fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + hash_map_allocation_bytes::<u64, u32>(self.primary.capacity())
            + hash_map_allocation_bytes::<u64, Vec<u32>>(self.collisions.capacity())
            + self
                .collisions
                .values()
                .map(|bucket| bucket.capacity() * std::mem::size_of::<u32>())
                .sum::<usize>()
    }
}

fn canonical_hex_fingerprint(id: &str) -> Option<u64> {
    let bytes: &[u8; 64] = id.as_bytes().try_into().ok()?;
    if !is_canonical_lower_hex(bytes) {
        return None;
    }
    Some(bytes[..16].iter().copied().fold(0_u64, |value, byte| {
        let nibble = (byte & 0x0f) + (byte >> 6) * 9;
        (value << 4) | u64::from(nibble)
    }))
}

#[cfg(target_arch = "x86_64")]
#[inline]
#[allow(clippy::cast_ptr_alignment)]
fn is_canonical_lower_hex(bytes: &[u8; 64]) -> bool {
    use std::arch::x86_64::{
        __m128i, _mm_and_si128, _mm_cmpgt_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_or_si128,
        _mm_set1_epi8,
    };

    // SAFETY: x86-64 guarantees SSE2. Each unaligned load stays within the
    // supplied 64-byte array, and these intrinsics do not mutate memory.
    unsafe {
        let slash = _mm_set1_epi8(b'/'.cast_signed());
        let colon = _mm_set1_epi8(b':'.cast_signed());
        let backtick = _mm_set1_epi8(b'`'.cast_signed());
        let g = _mm_set1_epi8(b'g'.cast_signed());
        for offset in [0, 16, 32, 48] {
            let characters = _mm_loadu_si128(bytes.as_ptr().add(offset).cast::<__m128i>());
            let decimal = _mm_and_si128(
                _mm_cmpgt_epi8(characters, slash),
                _mm_cmpgt_epi8(colon, characters),
            );
            let lower_hex = _mm_and_si128(
                _mm_cmpgt_epi8(characters, backtick),
                _mm_cmpgt_epi8(g, characters),
            );
            if _mm_movemask_epi8(_mm_or_si128(decimal, lower_hex)) != 0xffff {
                return false;
            }
        }
    }
    true
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn is_canonical_lower_hex(bytes: &[u8; 64]) -> bool {
    bytes
        .iter()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExtId(u32);

#[derive(Debug, Default)]
struct ExtensionInterner {
    ids: HashMap<String, ExtId>,
}

impl ExtensionInterner {
    fn intern(&mut self, extension: &str) -> ExtId {
        if let Some(id) = self.ids.get(extension) {
            return *id;
        }
        let id = ExtId(
            u32::try_from(self.ids.len())
                .expect("extension interner exceeded u32 capacity")
                .checked_add(1)
                .expect("extension interner exhausted reserved IDs"),
        );
        self.ids.insert(extension.to_owned(), id);
        id
    }

    fn matching_ids_ascii(&self, value: &str) -> Vec<ExtId> {
        let needle = value.to_ascii_lowercase();
        let mut matches = self
            .ids
            .iter()
            .filter_map(|(candidate, id)| {
                contains_ascii_lowercase(candidate, &needle).then_some(*id)
            })
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches
    }

    #[cfg(feature = "performance-test")]
    fn estimated_dynamic_bytes(&self) -> usize {
        hash_map_allocation_bytes::<String, ExtId>(self.ids.capacity())
            + self.ids.keys().map(String::capacity).sum::<usize>()
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
    pub thumbhash: Option<Box<[u8]>>,
    pub cache_version: u32,
    pub ext_id: ExtId,
    pub path_aliases: Vec<String>,
}

impl CacheRecord {
    pub fn from_abstract_data(data: &AbstractData, timestamp: i64, ext_id: ExtId) -> Self {
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
        Self {
            id: data.hash(),
            object_type,
            timestamp,
            width: data.width(),
            height: data.height(),
            size,
            thumbhash: data
                .thumbhash()
                .map(|value| value.clone().into_boxed_slice()),
            cache_version: data.cache_version(),
            ext_id,
            path_aliases: data
                .alias()
                .iter()
                .map(|alias| alias.file.clone())
                .collect(),
        }
    }

    fn from_abstract_data_owned(
        data: AbstractData,
        timestamp: i64,
        ext_id: ExtId,
    ) -> (Self, Option<AlbumCombined>) {
        match data {
            AbstractData::Image(image) => {
                crate::public::structure::object::observe_mutation_timestamp(
                    image.object.update_at,
                );
                let record = Self {
                    id: image.object.id,
                    object_type: ObjectType::Image,
                    timestamp,
                    width: image.metadata.width,
                    height: image.metadata.height,
                    size: image.metadata.size,
                    thumbhash: image.object.thumbhash.map(Vec::into_boxed_slice),
                    cache_version: image.object.cache_version,
                    ext_id,
                    path_aliases: image
                        .metadata
                        .alias
                        .into_iter()
                        .map(|alias| alias.file)
                        .collect(),
                };
                (record, None)
            }
            AbstractData::Video(video) => {
                crate::public::structure::object::observe_mutation_timestamp(
                    video.object.update_at,
                );
                let record = Self {
                    id: video.object.id,
                    object_type: ObjectType::Video,
                    timestamp,
                    width: video.metadata.width,
                    height: video.metadata.height,
                    size: video.metadata.size,
                    thumbhash: video.object.thumbhash.map(Vec::into_boxed_slice),
                    cache_version: video.object.cache_version,
                    ext_id,
                    path_aliases: video
                        .metadata
                        .alias
                        .into_iter()
                        .map(|alias| alias.file)
                        .collect(),
                };
                (record, None)
            }
            AbstractData::Album(album) => {
                crate::public::structure::object::observe_mutation_timestamp(
                    album.object.update_at,
                );
                let record = Self {
                    id: album.object.id,
                    object_type: ObjectType::Album,
                    timestamp,
                    width: 300,
                    height: 300,
                    size: 0,
                    thumbhash: album
                        .object
                        .thumbhash
                        .as_ref()
                        .map(|value| value.clone().into_boxed_slice()),
                    cache_version: album.object.cache_version,
                    ext_id,
                    path_aliases: Vec::new(),
                };
                (record, Some(album))
            }
        }
    }

    pub fn thumbhash_vec(&self) -> Option<Vec<u8>> {
        self.thumbhash.as_deref().map(<[u8]>::to_vec)
    }

    #[cfg(feature = "performance-test")]
    fn estimated_dynamic_bytes(&self) -> usize {
        self.thumbhash.as_ref().map_or(0, |value| value.len())
            + self.path_aliases.capacity() * std::mem::size_of::<String>()
            + self
                .path_aliases
                .iter()
                .map(String::capacity)
                .sum::<usize>()
    }
}

#[derive(Clone, Copy)]
struct AlbumMemberRef<'a> {
    id: ArrayString<64>,
    object_type: ObjectType,
    timestamp: i64,
    size: u64,
    thumbhash: Option<&'a [u8]>,
    cache_version: u32,
}

impl<'a> AlbumMemberRef<'a> {
    fn from_cache(record: &'a CacheRecord) -> Self {
        Self {
            id: record.id,
            object_type: record.object_type,
            timestamp: record.timestamp,
            size: record.size,
            thumbhash: record.thumbhash.as_deref(),
            cache_version: record.cache_version,
        }
    }

    fn from_abstract_data(data: &'a AbstractData, timestamp: i64) -> Self {
        let (object, object_type, size) = match data {
            AbstractData::Image(image) => (&image.object, ObjectType::Image, image.metadata.size),
            AbstractData::Video(video) => (&video.object, ObjectType::Video, video.metadata.size),
            AbstractData::Album(album) => (&album.object, ObjectType::Album, 0),
        };
        crate::public::structure::object::observe_mutation_timestamp(object.update_at);
        Self {
            id: object.id,
            object_type,
            timestamp,
            size,
            thumbhash: object.thumbhash.as_deref(),
            cache_version: object.cache_version,
        }
    }
}

struct AlbumAccumulator<'a> {
    current_cover: Option<ArrayString<64>>,
    item_count: usize,
    item_size: u64,
    start_time: Option<i64>,
    end_time: Option<i64>,
    current_cover_member: Option<AlbumMemberRef<'a>>,
    newest: Option<AlbumMemberRef<'a>>,
}

impl<'a> AlbumAccumulator<'a> {
    fn new(current_cover: Option<ArrayString<64>>) -> Self {
        Self {
            current_cover,
            item_count: 0,
            item_size: 0,
            start_time: None,
            end_time: None,
            current_cover_member: None,
            newest: None,
        }
    }

    fn push(&mut self, member: AlbumMemberRef<'a>) {
        if member.object_type == ObjectType::Album {
            return;
        }
        self.item_count += 1;
        self.item_size = self.item_size.saturating_add(member.size);
        self.start_time = Some(
            self.start_time
                .map_or(member.timestamp, |value| value.min(member.timestamp)),
        );
        self.end_time = Some(
            self.end_time
                .map_or(member.timestamp, |value| value.max(member.timestamp)),
        );
        if self.current_cover == Some(member.id) {
            self.current_cover_member = Some(member);
        }
        if self.newest.is_none_or(|candidate| {
            member.timestamp > candidate.timestamp
                || (member.timestamp == candidate.timestamp && member.id < candidate.id)
        }) {
            self.newest = Some(member);
        }
    }

    fn finish(self, mut album: AlbumCombined, changed_at: i64) -> AlbumCombined {
        album.metadata.item_count = self.item_count;
        album.metadata.item_size = self.item_size;
        album.metadata.start_time = self.start_time;
        album.metadata.end_time = self.end_time;
        album.metadata.last_modified_time = changed_at;
        album.object.touch_update_at(changed_at);

        if self.item_count == 0 {
            album.metadata.cover = None;
            album.object.thumbhash = None;
            album.object.cache_version = 0;
        } else if let Some(member) = self.current_cover_member.or(self.newest) {
            album.metadata.cover = Some(member.id);
            album.object.thumbhash = member.thumbhash.map(<[u8]>::to_vec);
            album.object.cache_version = member.cache_version;
        }
        album
    }
}

#[cfg(feature = "performance-test")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
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

    pub fn iter(&self) -> DenseBitmapIter<'_> {
        DenseBitmapIter::new(self)
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

#[derive(Debug, Clone)]
pub struct DenseBitmapIter<'a> {
    words: &'a [u64],
    word_index: usize,
    remaining_word: u64,
    remaining_len: usize,
}

impl<'a> DenseBitmapIter<'a> {
    fn new(bitmap: &'a DenseBitmap) -> Self {
        Self {
            words: &bitmap.words,
            word_index: 0,
            remaining_word: bitmap.words.first().copied().unwrap_or_default(),
            remaining_len: bitmap.len,
        }
    }
}

impl Iterator for DenseBitmapIter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining_word == 0 {
            self.word_index += 1;
            self.remaining_word = *self.words.get(self.word_index)?;
        }
        let bit = self.remaining_word.trailing_zeros();
        self.remaining_word &= self.remaining_word - 1;
        self.remaining_len -= 1;
        Some(u32::try_from(self.word_index).expect("bitmap index") * u64::BITS + bit)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining_len, Some(self.remaining_len))
    }
}

impl ExactSizeIterator for DenseBitmapIter<'_> {}
impl FusedIterator for DenseBitmapIter<'_> {}

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

pub enum OrdinalSetIter<'a> {
    Sparse(std::iter::Copied<std::slice::Iter<'a, u32>>),
    Dense(DenseBitmapIter<'a>),
}

impl Iterator for OrdinalSetIter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Sparse(iterator) => iterator.next(),
            Self::Dense(iterator) => iterator.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Sparse(iterator) => iterator.size_hint(),
            Self::Dense(iterator) => iterator.size_hint(),
        }
    }
}

impl ExactSizeIterator for OrdinalSetIter<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Sparse(iterator) => iterator.len(),
            Self::Dense(iterator) => iterator.len(),
        }
    }
}

impl FusedIterator for OrdinalSetIter<'_> {}

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

    pub fn intersects(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Sparse(left), Self::Sparse(right)) => {
                let (mut left_index, mut right_index) = (0, 0);
                while left_index < left.len() && right_index < right.len() {
                    match left[left_index].cmp(&right[right_index]) {
                        Ordering::Less => left_index += 1,
                        Ordering::Greater => right_index += 1,
                        Ordering::Equal => return true,
                    }
                }
                false
            }
            (Self::Dense(left), Self::Dense(right)) => left
                .words
                .iter()
                .zip(&right.words)
                .any(|(left, right)| left & right != 0),
            (Self::Sparse(sparse), Self::Dense(dense))
            | (Self::Dense(dense), Self::Sparse(sparse)) => {
                sparse.iter().any(|ordinal| dense.contains(*ordinal))
            }
        }
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

    #[cfg(test)]
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

    pub fn iter(&self) -> OrdinalSetIter<'_> {
        match self {
            Self::Sparse(items) => OrdinalSetIter::Sparse(items.iter().copied()),
            Self::Dense(bitmap) => OrdinalSetIter::Dense(bitmap.iter()),
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
                    if removed.get(removed_index) == Some(&item) {
                        changed(item);
                    } else {
                        next.push(item);
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
                // ExactSizeIterator lets `collect` reserve the exact current
                // cardinality. Preserve Vec's natural growth class so a facet
                // that later receives a few more members does not immediately
                // grow past the capacity used by the legacy iterator.
                let mut items = Vec::with_capacity(natural_vec_capacity::<u32>(bitmap.count()));
                items.extend(bitmap.iter());
                *self = Self::Sparse(items);
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

#[derive(Debug, Default)]
pub(crate) struct TargetSetBuilder {
    ordinals: DenseBitmap,
    generation_overrides: Vec<(u32, u32)>,
}

impl TargetSetBuilder {
    pub(crate) fn insert(&mut self, slot_ref: SlotRef) -> bool {
        if !self.ordinals.set(slot_ref.index(), true) {
            return false;
        }
        if slot_ref.generation() != 1 {
            self.generation_overrides
                .push((slot_ref.index(), slot_ref.generation()));
        }
        true
    }

    pub(crate) fn finish(mut self, universe: usize) -> TargetSet {
        self.generation_overrides
            .sort_unstable_by_key(|(ordinal, _)| *ordinal);
        let mut ordinals = OrdinalSet::Dense(self.ordinals);
        ordinals.rebalance(universe);
        TargetSet {
            ordinals,
            generation_overrides: self.generation_overrides,
        }
    }
}

pub enum TargetSetIter<'a> {
    GenerationOne(OrdinalSetIter<'a>),
    WithOverrides {
        ordinals: OrdinalSetIter<'a>,
        overrides: &'a [(u32, u32)],
        override_index: usize,
    },
}

impl Iterator for TargetSetIter<'_> {
    type Item = SlotRef;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::GenerationOne(ordinals) => {
                ordinals.next().map(|ordinal| SlotRef::new(ordinal, 1))
            }
            Self::WithOverrides {
                ordinals,
                overrides,
                override_index,
            } => {
                let ordinal = ordinals.next()?;
                while overrides
                    .get(*override_index)
                    .is_some_and(|(override_ordinal, _)| *override_ordinal < ordinal)
                {
                    *override_index += 1;
                }
                let generation = overrides
                    .get(*override_index)
                    .filter(|(override_ordinal, _)| *override_ordinal == ordinal)
                    .map_or(1, |(_, generation)| {
                        *override_index += 1;
                        *generation
                    });
                Some(SlotRef::new(ordinal, generation))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::GenerationOne(ordinals) | Self::WithOverrides { ordinals, .. } => {
                ordinals.size_hint()
            }
        }
    }
}

impl ExactSizeIterator for TargetSetIter<'_> {
    fn len(&self) -> usize {
        match self {
            Self::GenerationOne(ordinals) | Self::WithOverrides { ordinals, .. } => ordinals.len(),
        }
    }
}

impl FusedIterator for TargetSetIter<'_> {}

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
        let mut builder = TargetSetBuilder::default();
        for slot_ref in slot_refs {
            builder.insert(slot_ref);
        }
        builder.finish(universe)
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

    pub fn iter(&self) -> TargetSetIter<'_> {
        if self.generation_overrides.is_empty() {
            TargetSetIter::GenerationOne(self.ordinals.iter())
        } else {
            TargetSetIter::WithOverrides {
                ordinals: self.ordinals.iter(),
                overrides: &self.generation_overrides,
                override_index: 0,
            }
        }
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

    pub(crate) fn is_current(&self, state: &TreeState) -> bool {
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
struct CompactMembershipCounts {
    values: Vec<u16>,
    overflow: HashMap<u32, u32>,
}

impl CompactMembershipCounts {
    fn with_record_capacity(record_count: usize) -> Self {
        let mut values = Vec::with_capacity(natural_vec_capacity::<u16>(record_count));
        values.resize(record_count, 0);
        Self {
            values,
            overflow: HashMap::new(),
        }
    }

    fn ensure_ordinal(&mut self, ordinal: u32) {
        let len = ordinal as usize + 1;
        if self.values.len() < len {
            self.values.resize(len, 0);
        }
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn get(&self, ordinal: u32) -> u32 {
        self.overflow.get(&ordinal).copied().unwrap_or_else(|| {
            self.values
                .get(ordinal as usize)
                .copied()
                .unwrap_or_default()
                .into()
        })
    }

    fn set(&mut self, ordinal: u32, count: u32) {
        self.ensure_ordinal(ordinal);
        if count > u32::from(u16::MAX) {
            self.values[ordinal as usize] = u16::MAX;
            self.overflow.insert(ordinal, count);
        } else {
            self.values[ordinal as usize] =
                u16::try_from(count).expect("compact count fits after overflow check");
            self.overflow.remove(&ordinal);
        }
    }

    fn clear(&mut self, ordinal: u32) {
        self.set(ordinal, 0);
    }

    fn increment(&mut self, ordinal: u32) -> u32 {
        let count = self.get(ordinal).saturating_add(1);
        self.set(ordinal, count);
        count
    }

    fn decrement(&mut self, ordinal: u32) -> u32 {
        let count = self.get(ordinal).saturating_sub(1);
        self.set(ordinal, count);
        count
    }

    #[cfg(feature = "performance-test")]
    fn estimated_dynamic_bytes(&self) -> usize {
        self.values.capacity() * std::mem::size_of::<u16>()
            + hash_map_allocation_bytes::<u32, u32>(self.overflow.capacity())
    }
}

#[derive(Debug, Default)]
pub struct StringFacetIndex {
    values: HashMap<String, OrdinalSet>,
    has_any: DenseBitmap,
    membership_count: CompactMembershipCounts,
}

impl StringFacetIndex {
    pub fn get(&self, value: &str) -> Option<&OrdinalSet> {
        self.values.get(value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &OrdinalSet)> {
        self.values.iter()
    }

    pub fn has_any(&self) -> &DenseBitmap {
        &self.has_any
    }

    fn ensure_ordinal(&mut self, ordinal: u32) {
        self.membership_count.ensure_ordinal(ordinal);
    }

    fn insert_values<'a>(
        &mut self,
        ordinal: u32,
        values: impl IntoIterator<Item = &'a String>,
        universe: usize,
    ) {
        self.ensure_ordinal(ordinal);
        let mut count = 0_u32;
        for value in values {
            let inserted = if let Some(members) = self.values.get_mut(value) {
                members.insert(ordinal, universe)
            } else {
                let mut members = OrdinalSet::default();
                let inserted = members.insert(ordinal, universe);
                self.values.insert(value.clone(), members);
                inserted
            };
            if inserted {
                count = count.saturating_add(1);
            }
        }
        self.membership_count.set(ordinal, count);
        self.has_any.set(ordinal, count > 0);
    }

    #[cfg(all(test, feature = "performance-test"))]
    fn insert_values_legacy<'a>(
        &mut self,
        ordinal: u32,
        values: impl IntoIterator<Item = &'a String>,
        universe: usize,
    ) {
        self.ensure_ordinal(ordinal);
        let mut count = 0_u32;
        for value in values {
            if self
                .values
                .entry(value.clone())
                .or_default()
                .insert(ordinal, universe)
            {
                count = count.saturating_add(1);
            }
        }
        self.membership_count.set(ordinal, count);
        self.has_any.set(ordinal, count > 0);
    }

    #[cfg(test)]
    fn remove_record(&mut self, ordinal: u32, universe: usize) {
        self.ensure_ordinal(ordinal);
        self.membership_count.clear(ordinal);
        self.has_any.set(ordinal, false);
        self.values.retain(|_, members| {
            members.remove(ordinal, universe);
            !members.is_empty()
        });
    }

    fn remove_targets(&mut self, targets: &OrdinalSet, universe: usize) {
        for ordinal in targets.iter() {
            self.ensure_ordinal(ordinal);
            self.membership_count.clear(ordinal);
            self.has_any.set(ordinal, false);
        }
        self.values.retain(|_, members| {
            members.subtract(targets, universe);
            !members.is_empty()
        });
    }

    fn edit_values(
        &mut self,
        targets: &OrdinalSet,
        add: &BTreeSet<String>,
        remove: &BTreeSet<String>,
        universe: usize,
    ) {
        if let Some(max_ordinal) = targets.max() {
            self.ensure_ordinal(max_ordinal);
        }
        for value in add {
            let members = self.values.entry(value.clone()).or_default();
            let membership_count = &mut self.membership_count;
            let has_any = &mut self.has_any;
            members.union_with(targets, universe, |ordinal| {
                if membership_count.increment(ordinal) == 1 {
                    has_any.set(ordinal, true);
                }
            });
        }
        for value in remove {
            if let Some(members) = self.values.get_mut(value) {
                let membership_count = &mut self.membership_count;
                let has_any = &mut self.has_any;
                members.subtract_with(targets, universe, |ordinal| {
                    if membership_count.decrement(ordinal) == 0 {
                        has_any.set(ordinal, false);
                    }
                });
            }
        }
        self.values.retain(|_, members| !members.is_empty());
    }

    fn rebalance(&mut self, universe: usize) {
        for members in self.values.values_mut() {
            members.rebalance(universe);
        }
    }

    #[cfg(feature = "performance-test")]
    fn estimated_dynamic_bytes(&self) -> usize {
        self.has_any.words.capacity() * std::mem::size_of::<u64>()
            + hash_map_allocation_bytes::<String, OrdinalSet>(self.values.capacity())
            + self
                .values
                .iter()
                .map(|(value, members)| value.capacity() + members.estimated_bytes())
                .sum::<usize>()
            + self.membership_count.estimated_dynamic_bytes()
    }
}

#[derive(Debug, Default)]
pub struct SingleStringFacetIndex {
    values: HashMap<String, OrdinalSet>,
    has_any: DenseBitmap,
}

impl SingleStringFacetIndex {
    #[cfg(test)]
    pub fn get(&self, value: &str) -> Option<&OrdinalSet> {
        self.values.get(value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &OrdinalSet)> {
        self.values.iter()
    }

    pub fn has_any(&self) -> &DenseBitmap {
        &self.has_any
    }

    fn insert_value(&mut self, ordinal: u32, value: Option<&String>, universe: usize) {
        if let Some(value) = value {
            if let Some(members) = self.values.get_mut(value) {
                members.insert(ordinal, universe);
            } else {
                let mut members = OrdinalSet::default();
                members.insert(ordinal, universe);
                self.values.insert(value.clone(), members);
            }
            self.has_any.set(ordinal, true);
        } else {
            self.has_any.set(ordinal, false);
        }
    }

    #[cfg(all(test, feature = "performance-test"))]
    fn insert_value_legacy(&mut self, ordinal: u32, value: Option<&String>, universe: usize) {
        if let Some(value) = value {
            self.values
                .entry(value.clone())
                .or_default()
                .insert(ordinal, universe);
            self.has_any.set(ordinal, true);
        } else {
            self.has_any.set(ordinal, false);
        }
    }

    #[cfg(test)]
    fn remove_record(&mut self, ordinal: u32, universe: usize) {
        self.has_any.set(ordinal, false);
        self.values.retain(|_, members| {
            members.remove(ordinal, universe);
            !members.is_empty()
        });
    }

    fn remove_targets(&mut self, targets: &OrdinalSet, universe: usize) {
        for ordinal in targets.iter() {
            self.has_any.set(ordinal, false);
        }
        self.values.retain(|_, members| {
            members.subtract(targets, universe);
            !members.is_empty()
        });
    }

    fn matching_members_ascii(&self, value: &str, universe: usize) -> OrdinalSet {
        let needle = value.to_ascii_lowercase();
        let mut matches = OrdinalSet::default();
        for (candidate, members) in &self.values {
            if contains_ascii_lowercase(candidate, &needle) {
                matches.union_with(members, universe, |_| {});
            }
        }
        matches
    }

    fn rebalance(&mut self, universe: usize) {
        for members in self.values.values_mut() {
            members.rebalance(universe);
        }
    }

    #[cfg(feature = "performance-test")]
    fn estimated_dynamic_bytes(&self) -> usize {
        self.has_any.words.capacity() * std::mem::size_of::<u64>()
            + hash_map_allocation_bytes::<String, OrdinalSet>(self.values.capacity())
            + self
                .values
                .iter()
                .map(|(value, members)| value.capacity() + members.estimated_bytes())
                .sum::<usize>()
    }
}

#[derive(Debug, Default)]
pub struct QueryIndexes {
    pub favorite: DenseBitmap,
    pub archived: DenseBitmap,
    pub trashed: DenseBitmap,
    pub has_any_album: DenseBitmap,
    pub tags: StringFacetIndex,
    pub makes: SingleStringFacetIndex,
    pub models: SingleStringFacetIndex,
    pub albums: HashMap<ArrayString<64>, OrdinalSet>,
    album_membership_count: CompactMembershipCounts,
    extensions: ExtensionInterner,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FlagPatch {
    pub favorite: Option<bool>,
    pub archived: Option<bool>,
    pub trashed: Option<bool>,
}

impl QueryIndexes {
    fn with_record_capacity(record_count: usize) -> Self {
        Self {
            tags: StringFacetIndex {
                membership_count: CompactMembershipCounts::with_record_capacity(record_count),
                ..StringFacetIndex::default()
            },
            album_membership_count: CompactMembershipCounts::with_record_capacity(record_count),
            ..Self::default()
        }
    }

    #[cfg(feature = "performance-test")]
    fn estimated_bytes(&self) -> usize {
        let bitmap_bytes = [
            &self.favorite,
            &self.archived,
            &self.trashed,
            &self.has_any_album,
        ]
        .into_iter()
        .map(|bitmap| bitmap.words.capacity() * std::mem::size_of::<u64>())
        .sum::<usize>();
        let facet_bytes = self.tags.estimated_dynamic_bytes()
            + self.makes.estimated_dynamic_bytes()
            + self.models.estimated_dynamic_bytes()
            + self.extensions.estimated_dynamic_bytes();
        let album_bytes =
            hash_map_allocation_bytes::<ArrayString<64>, OrdinalSet>(self.albums.capacity())
                + self
                    .albums
                    .values()
                    .map(OrdinalSet::estimated_bytes)
                    .sum::<usize>();
        std::mem::size_of::<Self>()
            + bitmap_bytes
            + facet_bytes
            + album_bytes
            + self.album_membership_count.estimated_dynamic_bytes()
    }

    fn ensure_ordinal(&mut self, ordinal: u32) {
        self.album_membership_count.ensure_ordinal(ordinal);
    }

    fn insert_record(&mut self, ordinal: u32, data: &AbstractData, universe: usize) {
        self.ensure_ordinal(ordinal);
        let (object, tags, exif, albums) = match data {
            AbstractData::Image(image) => (
                &image.object,
                &image.object.tags,
                Some(&image.metadata.exif_vec),
                Some(&image.metadata.albums),
            ),
            AbstractData::Video(video) => (
                &video.object,
                &video.object.tags,
                Some(&video.metadata.exif_vec),
                Some(&video.metadata.albums),
            ),
            AbstractData::Album(album) => (&album.object, &album.object.tags, None, None),
        };
        self.favorite.set(ordinal, object.is_favorite);
        self.archived.set(ordinal, object.is_archived);
        self.trashed.set(ordinal, object.is_trashed);
        self.tags.insert_values(ordinal, tags.iter(), universe);
        self.makes.insert_value(
            ordinal,
            exif.and_then(|values| values.get("Make")),
            universe,
        );
        self.models.insert_value(
            ordinal,
            exif.and_then(|values| values.get("Model")),
            universe,
        );
        if let Some(albums) = albums {
            for album in albums {
                self.albums
                    .entry(*album)
                    .or_default()
                    .insert(ordinal, universe);
            }
            self.album_membership_count
                .set(ordinal, u32::try_from(albums.len()).unwrap_or(u32::MAX));
            self.has_any_album.set(ordinal, !albums.is_empty());
        } else {
            self.album_membership_count.clear(ordinal);
        }
    }

    #[cfg(all(test, feature = "performance-test"))]
    fn insert_record_legacy(&mut self, ordinal: u32, data: &AbstractData, universe: usize) {
        self.ensure_ordinal(ordinal);
        self.favorite.set(ordinal, object_flags(data).0);
        self.archived.set(ordinal, object_flags(data).1);
        self.trashed.set(ordinal, object_flags(data).2);
        self.tags
            .insert_values_legacy(ordinal, data.tag().iter(), universe);
        let exif = data.exif_vec();
        self.makes.insert_value_legacy(
            ordinal,
            exif.and_then(|values| values.get("Make")),
            universe,
        );
        self.models.insert_value_legacy(
            ordinal,
            exif.and_then(|values| values.get("Model")),
            universe,
        );
        if let Some(albums) = data.albums() {
            for album in albums {
                self.albums
                    .entry(*album)
                    .or_default()
                    .insert(ordinal, universe);
            }
            self.album_membership_count
                .set(ordinal, u32::try_from(albums.len()).unwrap_or(u32::MAX));
            self.has_any_album.set(ordinal, !albums.is_empty());
        } else {
            self.album_membership_count.clear(ordinal);
        }
    }

    #[cfg(test)]
    fn remove_record(&mut self, ordinal: u32, universe: usize) {
        self.favorite.set(ordinal, false);
        self.archived.set(ordinal, false);
        self.trashed.set(ordinal, false);
        self.has_any_album.set(ordinal, false);
        self.ensure_ordinal(ordinal);
        self.album_membership_count.clear(ordinal);
        self.tags.remove_record(ordinal, universe);
        self.makes.remove_record(ordinal, universe);
        self.models.remove_record(ordinal, universe);
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
            self.has_any_album.set(ordinal, false);
            self.ensure_ordinal(ordinal);
            self.album_membership_count.clear(ordinal);
        }
        self.tags.remove_targets(targets, universe);
        self.makes.remove_targets(targets, universe);
        self.models.remove_targets(targets, universe);
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
        self.tags.edit_values(targets, add, remove, universe);
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
                if membership_count.increment(ordinal) == 1 {
                    has_any.set(ordinal, true);
                }
            });
        }
        for album in remove {
            if let Some(members) = self.albums.get_mut(album) {
                let membership_count = &mut self.album_membership_count;
                let has_any = &mut self.has_any_album;
                members.subtract_with(targets, universe, |ordinal| {
                    if membership_count.decrement(ordinal) == 0 {
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

    fn matching_camera_members_ascii(&self, value: &str) -> OrdinalSet {
        let universe = self.album_membership_count.len();
        let mut matches = self.makes.matching_members_ascii(value, universe);
        let models = self.models.matching_members_ascii(value, universe);
        matches.union_with(&models, universe, |_| {});
        matches
    }

    fn matching_make_members_ascii(&self, value: &str) -> OrdinalSet {
        self.makes
            .matching_members_ascii(value, self.album_membership_count.len())
    }

    fn matching_model_members_ascii(&self, value: &str) -> OrdinalSet {
        self.models
            .matching_members_ascii(value, self.album_membership_count.len())
    }

    fn intern_extension(&mut self, extension: &str) -> ExtId {
        self.extensions.intern(extension)
    }

    fn matching_extension_ids_ascii(&self, value: &str) -> Vec<ExtId> {
        self.extensions.matching_ids_ascii(value)
    }

    fn finish_rebuild(&mut self, universe: usize) {
        self.tags.rebalance(universe);
        self.makes.rebalance(universe);
        self.models.rebalance(universe);
        for members in self.albums.values_mut() {
            members.rebalance(universe);
        }
    }
}

#[cfg(all(test, feature = "performance-test"))]
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
    // Cache revision: changes for inserts, removals, and order changes.
    structural_epoch: u64,
    // Slot-reference namespace: changes only when the complete tree is rebuilt.
    identity_epoch: u64,
    // Destructive revision: changes when an existing slot identity is removed.
    selection_epoch: u64,
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
            structural_epoch: next_tree_epoch(),
            identity_epoch: next_tree_epoch(),
            selection_epoch: next_tree_epoch(),
        }
    }
}

impl TreeState {
    fn with_record_capacity(record_count: usize) -> Self {
        assert!(
            u32::try_from(record_count).is_ok(),
            "record arena exceeded u32 capacity"
        );
        Self {
            arena: RecordArena::with_record_capacity(record_count),
            id_index: IdIndex::with_record_capacity(record_count),
            order: Arc::default(),
            query: QueryIndexes::with_record_capacity(record_count),
            albums: HashMap::new(),
            #[cfg(feature = "performance-test")]
            record_dynamic_bytes: 0,
            structural_epoch: next_tree_epoch(),
            identity_epoch: next_tree_epoch(),
            selection_epoch: next_tree_epoch(),
        }
    }

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

    #[cfg(test)]
    pub fn from_records(records: impl IntoIterator<Item = AbstractData>) -> Self {
        let records = records.into_iter();
        let record_count = exact_size_hint(&records);
        let mut state = Self::with_record_capacity(record_count);
        let mut order = Vec::with_capacity(natural_vec_capacity::<SlotRef>(record_count));
        for data in records {
            order.push(state.push_unsorted_owned(data));
        }
        state.query.finish_rebuild(state.arena.capacity());
        state.finish_unsorted_order(order)
    }

    // Compatibility constructor retained for callers that do not have a
    // trustworthy record-count hint.
    #[allow(dead_code)]
    pub fn try_from_records<E>(
        records: impl IntoIterator<Item = Result<AbstractData, E>>,
    ) -> Result<Self, E> {
        let records = records.into_iter();
        let record_count = exact_size_hint(&records);
        Self::try_from_records_with_capacity(records, record_count)
    }

    pub(crate) fn try_from_records_with_capacity<E>(
        records: impl IntoIterator<Item = Result<AbstractData, E>>,
        record_count: usize,
    ) -> Result<Self, E> {
        let mut state = Self::with_record_capacity(record_count);
        let mut order = Vec::with_capacity(natural_vec_capacity::<SlotRef>(record_count));
        for data in records {
            order.push(state.push_unsorted_owned(data?));
        }
        state.query.finish_rebuild(state.arena.capacity());
        Ok(state.finish_unsorted_order(order))
    }

    #[cfg(all(test, feature = "performance-test"))]
    fn from_records_legacy_owned(records: Vec<AbstractData>) -> Self {
        let mut state = Self::default();
        for data in records {
            state.push_unsorted_borrowed_legacy(&data);
        }
        state.finish_unsorted()
    }

    #[cfg(all(test, feature = "performance-test"))]
    fn try_from_records_legacy_owned<E>(
        records: impl IntoIterator<Item = Result<AbstractData, E>>,
    ) -> Result<Self, E> {
        let mut state = Self::default();
        for data in records {
            let data = data?;
            state.push_unsorted_borrowed_legacy(&data);
        }
        Ok(state.finish_unsorted())
    }

    #[cfg(all(test, feature = "performance-test"))]
    fn push_unsorted_borrowed_legacy(&mut self, data: &AbstractData) {
        let timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
        let ext_id = self.query.intern_extension(data.ext());
        let record = CacheRecord::from_abstract_data(data, timestamp, ext_id);
        self.track_record_added(&record);
        let id = record.id;
        let slot_ref = self.arena.allocate(record);
        self.id_index.insert(id.as_str(), slot_ref);
        if let AbstractData::Album(album) = data {
            self.albums.insert(album.object.id, album.clone());
        }
        let universe = self.arena.capacity();
        self.query
            .insert_record_legacy(slot_ref.index(), data, universe);
        Arc::make_mut(&mut self.order).push(slot_ref);
    }

    fn push_unsorted_owned(&mut self, data: AbstractData) -> SlotRef {
        let timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
        let ext_id = self.query.intern_extension(data.ext());
        let ordinal =
            u32::try_from(self.arena.capacity()).expect("record arena exceeded u32 capacity");
        let universe = self.arena.capacity().saturating_add(1);
        self.query.insert_record(ordinal, &data, universe);
        let (record, album) = CacheRecord::from_abstract_data_owned(data, timestamp, ext_id);
        self.track_record_added(&record);
        let id = record.id;
        let slot_ref = self.arena.allocate(record);
        debug_assert_eq!(slot_ref.index(), ordinal);
        self.id_index.insert(id.as_str(), slot_ref);
        if let Some(album) = album {
            self.albums.insert(album.object.id, album);
        }
        slot_ref
    }

    #[cfg(all(test, feature = "performance-test"))]
    fn finish_unsorted(mut self) -> Self {
        let arena = &self.arena;
        Arc::make_mut(&mut self.order)
            .sort_unstable_by(|left, right| compare_slots(arena, *left, *right));
        self
    }

    fn finish_unsorted_order(mut self, mut order: Vec<SlotRef>) -> Self {
        let arena = &self.arena;
        order.sort_unstable_by(|left, right| compare_occupied_slots(arena, *left, *right));
        self.order = Arc::new(order);
        self
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

    #[allow(clippy::unused_self)]
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

    #[allow(clippy::unused_self)]
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

    pub fn identity_epoch(&self) -> u64 {
        self.identity_epoch
    }

    pub fn selection_epoch(&self) -> u64 {
        self.selection_epoch
    }

    fn bump_structural_epoch(&mut self) {
        self.structural_epoch = next_tree_epoch();
    }

    fn bump_selection_epoch(&mut self) {
        self.selection_epoch = next_tree_epoch();
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

    pub fn insert(&mut self, data: &AbstractData) -> SlotRef {
        let timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
        let ext_id = self.query.intern_extension(data.ext());
        let record = CacheRecord::from_abstract_data(data, timestamp, ext_id);
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

    #[cfg(test)]
    pub fn remove(&mut self, slot_ref: SlotRef) -> Option<CacheRecord> {
        let id = self.arena.get(slot_ref)?.id;
        self.id_index.remove(id.as_str(), slot_ref, &self.arena);
        self.query
            .remove_record(slot_ref.index(), self.arena.capacity());
        self.albums.remove(&id);
        Arc::make_mut(&mut self.order).retain(|candidate| *candidate != slot_ref);
        let removed = self.arena.remove(slot_ref);
        if let Some(record) = &removed {
            self.track_record_removed(record);
            self.bump_structural_epoch();
            self.bump_selection_epoch();
        }
        removed
    }

    pub fn remove_targets(&mut self, targets: &TargetSet) {
        if targets.is_empty() {
            return;
        }
        let universe = self.arena.capacity();
        self.query.remove_targets(targets.ordinals(), universe);
        let mut removed_any = false;
        for slot_ref in targets.iter() {
            let Some(id) = self.arena.get(slot_ref).map(|record| record.id) else {
                continue;
            };
            self.id_index.remove(id.as_str(), slot_ref, &self.arena);
            self.albums.remove(&id);
            if let Some(record) = self.arena.remove(slot_ref) {
                self.track_record_removed(&record);
                removed_any = true;
            }
        }
        let next_order = self
            .order
            .iter()
            .copied()
            .filter(|slot_ref| !targets.contains(*slot_ref))
            .collect();
        self.order = Arc::new(next_order);
        if removed_any {
            self.bump_structural_epoch();
            self.bump_selection_epoch();
        }
    }

    #[cfg(test)]
    pub fn replace_static(&mut self, slot_ref: SlotRef, data: &AbstractData) -> Option<()> {
        let old = self.arena.get(slot_ref)?.clone();
        if old.id != data.hash() {
            return None;
        }
        let new_timestamp = data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST);
        let ext_id = self.query.intern_extension(data.ext());
        let new = CacheRecord::from_abstract_data(data, new_timestamp, ext_id);
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
            self.id_index.remove(id.as_str(), *slot_ref, &self.arena);
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
            let ext_id = self.query.intern_extension(data.ext());
            let next_record = CacheRecord::from_abstract_data(data, timestamp, ext_id);
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
        if !removed_slots.is_empty() {
            self.bump_selection_epoch();
        }
    }

    #[cfg(test)]
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

    #[allow(clippy::too_many_lines)]
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
                        record.thumbhash_vec(),
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

    #[allow(clippy::too_many_lines)]
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
            .filter(|(_, members)| targets.intersects(members))
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
                            record.thumbhash_vec(),
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
        let album = self.albums.get(&album_id)?.clone();
        let mut accumulator = AlbumAccumulator::new(album.metadata.cover);
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
        {
            accumulator.push(AlbumMemberRef::from_cache(record));
        }
        Some(accumulator.finish(album, changed_at))
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
        let album = self.albums.get(&album_id)?.clone();
        let override_record = AlbumMemberRef::from_abstract_data(
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
        let mut accumulator = AlbumAccumulator::new(album.metadata.cover);
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
                    accumulator.push(override_record);
                }
                continue;
            }
            if self.query.trashed.contains(slot_ref.index()) {
                continue;
            }
            if let Some(record) = self.get(slot_ref) {
                accumulator.push(AlbumMemberRef::from_cache(record));
            }
        }
        if override_is_member && !override_is_trashed && !saw_override {
            accumulator.push(override_record);
        }
        Some(accumulator.finish(album, changed_at))
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

fn compare_occupied_slots(
    arena: &RecordArena<CacheRecord>,
    left: SlotRef,
    right: SlotRef,
) -> Ordering {
    let left = arena.slots[left.index() as usize]
        .value
        .as_ref()
        .expect("rebuild order only contains occupied slots");
    let right = arena.slots[right.index() as usize]
        .value
        .as_ref()
        .expect("rebuild order only contains occupied slots");
    right
        .timestamp
        .cmp(&left.timestamp)
        .then_with(|| left.id.cmp(&right.id))
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
        if compare_slots(arena, existing[left], additions[right]) == Ordering::Greater {
            merged.push(additions[right]);
            right += 1;
        } else {
            merged.push(existing[left]);
            left += 1;
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
    OwnedMembership(OrdinalSet),
    Bitmap(&'a DenseBitmap, bool),
    ExtType {
        image: bool,
        video: bool,
        album: bool,
    },
    Ext(Vec<ExtId>),
    Path(String),
    HiddenAlbumExists(bool),
    Any {
        needle: String,
        camera_members: OrdinalSet,
        extension_ids: Vec<ExtId>,
        tag_members: Option<&'a OrdinalSet>,
        include_paths: bool,
        include_album_type: bool,
    },
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
            Expression::Tag(_) | Expression::Path(_) if hidden_metadata_album.is_some() => {
                Self::Constant(false)
            }
            Expression::Tag(FilterValue::Value(tag)) => Self::Membership(indexes.tags.get(tag)),
            Expression::Tag(FilterValue::Exists(exists)) => {
                Self::Bitmap(indexes.tags.has_any(), *exists)
            }
            Expression::Favorite(value) => Self::Bitmap(&indexes.favorite, *value),
            Expression::Archived(value) => Self::Bitmap(&indexes.archived, *value),
            Expression::Trashed(value) => Self::Bitmap(&indexes.trashed, *value),
            Expression::ExtType(ext_type) => Self::ExtType {
                image: ext_type.contains("image"),
                video: ext_type.contains("video"),
                album: hidden_metadata_album.is_none() && ext_type.contains("album"),
            },
            Expression::Ext(ext) => Self::Ext(indexes.matching_extension_ids_ascii(ext)),
            Expression::Model(FilterValue::Value(model)) => {
                Self::OwnedMembership(indexes.matching_model_members_ascii(model))
            }
            Expression::Model(FilterValue::Exists(exists)) => {
                Self::Bitmap(indexes.models.has_any(), *exists)
            }
            Expression::Make(FilterValue::Value(make)) => {
                Self::OwnedMembership(indexes.matching_make_members_ascii(make))
            }
            Expression::Make(FilterValue::Exists(exists)) => {
                Self::Bitmap(indexes.makes.has_any(), *exists)
            }
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
                camera_members: indexes.matching_camera_members_ascii(value),
                extension_ids: indexes.matching_extension_ids_ascii(value),
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
            Self::OwnedMembership(members) => members.contains(ordinal),
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
            Self::Ext(extension_ids) => extension_ids.binary_search(&record.ext_id).is_ok(),
            Self::Path(needle) => record
                .path_aliases
                .iter()
                .any(|path| contains_ascii_lowercase(path, needle)),
            Self::HiddenAlbumExists(exists) => record.object_type != ObjectType::Album && *exists,
            Self::Any {
                needle,
                camera_members,
                extension_ids,
                tag_members,
                include_paths,
                include_album_type,
            } => {
                contains_ascii_lowercase(record.id.as_str(), needle)
                    || extension_ids.binary_search(&record.ext_id).is_ok()
                    || camera_members.contains(ordinal)
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
            ext_id: ExtId::default(),
            path_aliases: Vec::new(),
        }
    }

    fn naive_finish_album(
        mut album: AlbumCombined,
        records: &[CacheRecord],
        changed_at: i64,
    ) -> AlbumCombined {
        let current_cover = album.metadata.cover;
        let mut item_count = 0_usize;
        let mut item_size = 0_u64;
        let mut start_time = None::<i64>;
        let mut end_time = None::<i64>;
        let mut cover = None::<&CacheRecord>;
        let mut newest = None::<&CacheRecord>;
        for record in records
            .iter()
            .filter(|record| record.object_type != ObjectType::Album)
        {
            item_count += 1;
            item_size = item_size.saturating_add(record.size);
            start_time =
                Some(start_time.map_or(record.timestamp, |value| value.min(record.timestamp)));
            end_time = Some(end_time.map_or(record.timestamp, |value| value.max(record.timestamp)));
            if current_cover == Some(record.id) {
                cover = Some(record);
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
        } else if let Some(record) = cover.or(newest) {
            album.metadata.cover = Some(record.id);
            album.object.thumbhash = record.thumbhash_vec();
            album.object.cache_version = record.cache_version;
        }
        album
    }

    fn naive_album_aggregate_excluding(
        state: &TreeState,
        album_id: ArrayString<64>,
        excluded: &TargetSet,
        changed_at: i64,
    ) -> Option<AlbumCombined> {
        let album = state.albums.get(&album_id)?.clone();
        let records = state
            .query
            .albums
            .get(&album_id)
            .into_iter()
            .flat_map(|members| members.iter())
            .filter(|ordinal| !state.query.trashed.contains(*ordinal))
            .filter_map(|ordinal| state.slot_for_ordinal(ordinal))
            .filter(|slot_ref| !excluded.contains(*slot_ref))
            .filter_map(|slot_ref| state.get(slot_ref))
            .cloned()
            .collect::<Vec<_>>();
        Some(naive_finish_album(album, &records, changed_at))
    }

    fn naive_album_aggregate_with_override(
        state: &TreeState,
        album_id: ArrayString<64>,
        override_slot: SlotRef,
        data: &AbstractData,
        changed_at: i64,
    ) -> Option<AlbumCombined> {
        let album = state.albums.get(&album_id)?.clone();
        let override_record = CacheRecord::from_abstract_data(
            data,
            data.compute_timestamp(crate::public::constant::DEFAULT_PRIORITY_LIST),
            ExtId::default(),
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
        for slot_ref in state
            .query
            .albums
            .get(&album_id)
            .into_iter()
            .flat_map(|members| members.iter())
            .filter_map(|ordinal| state.slot_for_ordinal(ordinal))
        {
            if slot_ref == override_slot {
                saw_override = true;
                if override_is_member && !override_is_trashed {
                    records.push(override_record.clone());
                }
                continue;
            }
            if state.query.trashed.contains(slot_ref.index()) {
                continue;
            }
            if let Some(record) = state.get(slot_ref) {
                records.push(record.clone());
            }
        }
        if override_is_member && !override_is_trashed && !saw_override {
            records.push(override_record);
        }
        Some(naive_finish_album(album, &records, changed_at))
    }

    fn assert_album_eq(left: &AlbumCombined, right: &AlbumCombined) {
        assert_eq!(left.object, right.object);
        assert_eq!(left.metadata, right.metadata);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn cache_record_layout_stays_compact() {
        assert_eq!(std::mem::size_of::<CacheRecord>(), 144);
        assert_eq!(std::mem::size_of::<ArenaSlot<CacheRecord>>(), 152);
    }

    #[test]
    fn owned_media_cache_record_reuses_thumbhash_and_path_allocations() {
        let id = ArrayString::<64>::from("owned-media").unwrap();
        let mut metadata =
            crate::public::structure::image::ImageMetadata::new(id, 9, 10, 11, "jpg".to_owned());
        let mut path = String::with_capacity(128);
        path.push_str("C:/gallery/owned-media.jpg");
        metadata
            .alias
            .push(crate::public::structure::common::FileModify {
                file: path,
                modified: 1,
                scan_time: 2,
            });
        let mut object = ObjectSchema::new(id, ObjectType::Image);
        object.thumbhash = Some(vec![1, 2, 3, 4, 5]);
        let data = AbstractData::Image(crate::public::structure::image::ImageCombined {
            object,
            metadata,
        });
        let path_pointer = data.alias()[0].file.as_ptr();
        let thumbhash_pointer = data.thumbhash().unwrap().as_ptr();

        let (record, album) = CacheRecord::from_abstract_data_owned(data, 123, ExtId::default());

        assert!(album.is_none());
        assert_eq!(record.path_aliases[0].as_ptr(), path_pointer);
        assert_eq!(
            record.thumbhash.as_ref().unwrap().as_ptr(),
            thumbhash_pointer
        );
    }

    fn camera_record(index: u64, make: &str, model: &str) -> AbstractData {
        let id = ArrayString::<64>::from(format!("camera-{index}").as_str()).unwrap();
        let mut metadata =
            crate::public::structure::image::ImageMetadata::new(id, 1, 1, 1, "jpg".to_owned());
        metadata.exif_vec.insert("Make".to_owned(), make.to_owned());
        metadata
            .exif_vec
            .insert("Model".to_owned(), model.to_owned());
        AbstractData::Image(crate::public::structure::image::ImageCombined {
            object: ObjectSchema::new(id, ObjectType::Image),
            metadata,
        })
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
        let mut random = 0x0B01_D5E7_u64;
        let mut actual = OrdinalSet::default();
        let mut expected = BTreeSet::new();
        for _ in 0..250 {
            let mut values = BTreeSet::new();
            for _ in 0..128 {
                random = random
                    .wrapping_mul(2_862_933_555_777_941_757)
                    .wrapping_add(3_037_000_493);
                values.insert(u32::try_from(random % u64::try_from(universe).unwrap()).unwrap());
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
        assert_eq!(indexes.tags.get("first").unwrap().len(), 128);
        assert_eq!(indexes.tags.get("second").unwrap().len(), 128);
        assert_eq!(indexes.tags.membership_count.get(63), 1);
        assert_eq!(indexes.tags.membership_count.get(64), 2);
        assert!(indexes.tags.has_any().contains(191));

        indexes.edit_tags(
            &targets,
            &BTreeSet::new(),
            &BTreeSet::from(["first".to_owned(), "second".to_owned()]),
            universe,
        );
        assert!(!indexes.tags.has_any().contains(63));
        assert!(!indexes.tags.has_any().contains(64));
        assert!(indexes.tags.has_any().contains(128));
        assert_eq!(indexes.tags.membership_count.get(64), 0);
        assert_eq!(indexes.tags.membership_count.get(128), 1);
    }

    #[test]
    fn compact_membership_counts_preserve_overflow_boundaries() {
        let mut counts = CompactMembershipCounts::default();
        counts.set(7, u32::from(u16::MAX) - 1);
        assert_eq!(counts.increment(7), u32::from(u16::MAX));
        assert!(!counts.overflow.contains_key(&7));

        assert_eq!(counts.increment(7), u32::from(u16::MAX) + 1);
        assert_eq!(counts.overflow.get(&7), Some(&(u32::from(u16::MAX) + 1)));

        assert_eq!(counts.decrement(7), u32::from(u16::MAX));
        assert!(!counts.overflow.contains_key(&7));
        assert_eq!(counts.decrement(7), u32::from(u16::MAX) - 1);

        counts.clear(7);
        assert_eq!(counts.get(7), 0);
    }

    #[test]
    fn camera_facets_are_independent_and_follow_static_record_lifecycle() {
        let first = camera_record(10, "Canon", "R5");
        let second = camera_record(11, "CANON", "R6");
        let first_id = first.hash();
        let second_id = second.hash();
        let mut state = TreeState::from_records([first.clone(), second]);
        let first_slot = state.find(first_id.as_str()).unwrap();
        let second_slot = state.find(second_id.as_str()).unwrap();

        assert_eq!(state.query.makes.get("Canon").unwrap().len(), 1);
        assert_eq!(state.query.makes.get("CANON").unwrap().len(), 1);
        assert_eq!(state.query.models.get("R5").unwrap().len(), 1);
        assert_eq!(state.query.models.get("R6").unwrap().len(), 1);
        assert!(state.query.makes.has_any().contains(first_slot.index()));
        assert!(state.query.models.has_any().contains(second_slot.index()));

        let make = Expression::Make(FilterValue::Value("canon".to_owned()));
        assert!(state.matches(first_slot, &make, None));
        assert!(state.matches(second_slot, &make, None));
        let any_make = Expression::Any("canon".to_owned());
        assert!(state.matches(first_slot, &any_make, None));
        assert!(state.matches(second_slot, &any_make, None));
        let any_model = Expression::Any("r6".to_owned());
        assert!(!state.matches(first_slot, &any_model, None));
        assert!(state.matches(second_slot, &any_model, None));

        let impossible_pair = Expression::And(vec![
            Expression::Make(FilterValue::Value("Canon".to_owned())),
            Expression::Model(FilterValue::Value("R6".to_owned())),
        ]);
        assert!(!state.matches(first_slot, &impossible_pair, None));
        assert!(state.matches(second_slot, &impossible_pair, None));

        let mut replacement = first;
        let exif = replacement.exif_vec_mut().unwrap();
        exif.insert("Make".to_owned(), "Sony".to_owned());
        exif.insert("Model".to_owned(), "A1".to_owned());
        state.replace_static(first_slot, &replacement).unwrap();
        assert!(state.query.makes.get("Canon").is_none());
        assert!(state.query.models.get("R5").is_none());
        assert!(
            state
                .query
                .makes
                .get("Sony")
                .unwrap()
                .contains(first_slot.index())
        );
        assert!(
            state
                .query
                .models
                .get("A1")
                .unwrap()
                .contains(first_slot.index())
        );
        assert!(state.matches(first_slot, &Expression::Any("sony".to_owned()), None));
        assert!(!state.matches(first_slot, &Expression::Any("canon".to_owned()), None));

        state.remove(second_slot).unwrap();
        assert!(state.query.makes.get("CANON").is_none());
        assert!(state.query.models.get("R6").is_none());
    }

    #[test]
    fn extension_ids_preserve_case_insensitive_query_and_album_semantics() {
        let mut mixed_case = camera_record(20, "", "");
        let mut lowercase = camera_record(21, "", "");
        let mut empty = camera_record(22, "", "");
        if let AbstractData::Image(image) = &mut mixed_case {
            image.metadata.ext = "JpG".to_owned();
        }
        if let AbstractData::Image(image) = &mut lowercase {
            image.metadata.ext = "jpg".to_owned();
        }
        if let AbstractData::Image(image) = &mut empty {
            image.metadata.ext.clear();
        }
        let album_id = ArrayString::<64>::from("extension-album").unwrap();
        let album = Album::new(album_id, None).into_abstract_data();
        let mixed_id = mixed_case.hash();
        let lowercase_id = lowercase.hash();
        let empty_id = empty.hash();
        let mut state = TreeState::from_records([mixed_case.clone(), lowercase, empty, album]);
        let mixed_slot = state.find(mixed_id.as_str()).unwrap();
        let lowercase_slot = state.find(lowercase_id.as_str()).unwrap();
        let empty_slot = state.find(empty_id.as_str()).unwrap();
        let album_slot = state.find(album_id.as_str()).unwrap();

        assert_ne!(
            state.get(mixed_slot).unwrap().ext_id,
            state.get(lowercase_slot).unwrap().ext_id
        );
        let jpg = Expression::Ext("JPG".to_owned());
        assert!(state.matches(mixed_slot, &jpg, None));
        assert!(state.matches(lowercase_slot, &jpg, None));
        assert!(!state.matches(empty_slot, &jpg, None));
        assert!(!state.matches(album_slot, &jpg, None));
        assert!(state.matches(mixed_slot, &Expression::Any("pG".to_owned()), None));

        let empty_query = Expression::Ext(String::new());
        for slot_ref in [mixed_slot, lowercase_slot, empty_slot, album_slot] {
            assert!(state.matches(slot_ref, &empty_query, None));
        }

        if let AbstractData::Image(image) = &mut mixed_case {
            image.metadata.ext = "png".to_owned();
        }
        state.replace_static(mixed_slot, &mixed_case).unwrap();
        assert!(!state.matches(mixed_slot, &jpg, None));
        assert!(state.matches(mixed_slot, &Expression::Ext("PnG".to_owned()), None));

        state.remove(lowercase_slot).unwrap();
        assert!(!state.matches(mixed_slot, &jpg, None));
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
    #[allow(clippy::too_many_lines)]
    fn streaming_album_aggregates_match_randomized_naive_oracle() {
        let album_id = ArrayString::<64>::from("album-streaming-oracle").unwrap();
        let media = (0..96_u64)
            .map(|index| {
                let id =
                    ArrayString::<64>::from(format!("oracle-media-{index:03}").as_str()).unwrap();
                let mut object = ObjectSchema::new(
                    id,
                    if index % 2 == 0 {
                        ObjectType::Image
                    } else {
                        ObjectType::Video
                    },
                );
                object.is_trashed = index % 7 == 0;
                object.cache_version = u32::try_from(index).unwrap();
                if index % 4 == 0 {
                    object.thumbhash = Some(vec![u8::try_from(index).unwrap(), 0xA5]);
                }
                let size = if index == 5 {
                    u64::MAX - 3
                } else {
                    index * 17 + 1
                };
                let date = format!(
                    "2020-{:02}-{:02} {:02}:{:02}:{:02}",
                    1 + index % 12,
                    1 + index % 28,
                    index % 6,
                    index % 10,
                    index % 10
                );
                if index % 2 == 0 {
                    let mut metadata = crate::public::structure::image::ImageMetadata::new(
                        id,
                        size,
                        10,
                        20,
                        "jpg".to_owned(),
                    );
                    if index % 3 != 0 {
                        metadata.albums.insert(album_id);
                    }
                    metadata
                        .exif_vec
                        .insert("DateTimeOriginal".to_owned(), date);
                    AbstractData::Image(crate::public::structure::image::ImageCombined {
                        object,
                        metadata,
                    })
                } else {
                    let mut metadata = crate::public::structure::video::VideoMetadata::new(
                        id,
                        size,
                        10,
                        20,
                        "mp4".to_owned(),
                    );
                    if index % 3 != 0 {
                        metadata.albums.insert(album_id);
                    }
                    metadata
                        .exif_vec
                        .insert("DateTimeOriginal".to_owned(), date);
                    AbstractData::Video(crate::public::structure::video::VideoCombined {
                        object,
                        metadata,
                    })
                }
            })
            .collect::<Vec<_>>();
        let album = Album::new(album_id, None).into_abstract_data();
        let mut records = media.clone();
        records.push(album);
        let mut state = TreeState::from_records(records);
        let slots = media
            .iter()
            .map(|record| state.find(record.hash().as_str()).unwrap())
            .collect::<Vec<_>>();

        // A malformed self-membership remains excluded just like any other
        // Album object; this also exercises the accumulator's type guard.
        let album_slot = state.find(album_id.as_str()).unwrap();
        let universe = state.arena.capacity();
        state
            .query
            .albums
            .entry(album_id)
            .or_default()
            .insert(album_slot.index(), universe);

        let missing_cover = ArrayString::<64>::from("missing-cover").unwrap();
        let mut random = 0x9E37_79B9_7F4A_7C15_u64;
        for round in 0..128_usize {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            state.albums.get_mut(&album_id).unwrap().metadata.cover = match round % 4 {
                0 => None,
                1 => Some(media[round % media.len()].hash()),
                2 => Some(missing_cover),
                _ => Some(media[7].hash()),
            };

            let excluded_slots = if round % 11 == 0 {
                slots.clone()
            } else {
                slots
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(index, _)| (random >> (index % 32)).trailing_zeros() >= 3)
                    .map(|(_, slot_ref)| slot_ref)
                    .collect()
            };
            let excluded = TargetSet::from_slot_refs(excluded_slots, universe);
            let changed_at = crate::public::structure::object::next_mutation_timestamp();
            let expected =
                naive_album_aggregate_excluding(&state, album_id, &excluded, changed_at).unwrap();
            let actual = state
                .album_aggregate_excluding(album_id, &excluded, changed_at)
                .unwrap();
            assert_album_eq(&actual, &expected);

            let selected = usize::try_from(random % u64::try_from(media.len()).unwrap()).unwrap();
            let mut updated = media[selected].clone();
            if random & 1 == 0 {
                updated.albums_mut().unwrap().insert(album_id);
            } else {
                updated.albums_mut().unwrap().remove(&album_id);
            }
            updated.set_trashed(random & 2 != 0);
            updated.set_size(if random & 4 == 0 {
                u64::MAX
            } else {
                random & 0x00ff_ffff
            });
            updated.set_thumbhash(vec![
                u8::try_from(round).unwrap(),
                u8::try_from((random >> 8) & 0xff).unwrap(),
                0x5A,
            ]);
            updated.set_cache_version(u32::try_from(round).unwrap() + 1000);
            if let Some(exif) = updated.exif_vec_mut() {
                exif.insert(
                    "DateTimeOriginal".to_owned(),
                    format!("2021-01-01 00:{:02}:{:02}", round % 60, selected % 60),
                );
            }
            let changed_at = crate::public::structure::object::next_mutation_timestamp();
            let expected = naive_album_aggregate_with_override(
                &state,
                album_id,
                slots[selected],
                &updated,
                changed_at,
            )
            .unwrap();
            let actual = state
                .album_aggregate_with_override(album_id, slots[selected], &updated, changed_at)
                .unwrap();
            assert_album_eq(&actual, &expected);
        }
    }

    #[test]
    fn album_accumulator_scratch_remains_constant_for_one_million_members() {
        const MEMBER_COUNT: usize = 1_000_000;
        assert!(std::mem::size_of::<AlbumAccumulator<'static>>() <= 64 * 1024);

        let mut record = cache_record("constant-album-member", 123);
        record.size = u64::MAX;
        record.thumbhash = Some(vec![1, 2, 3].into_boxed_slice());
        record.cache_version = 9;
        let mut accumulator = AlbumAccumulator::new(Some(record.id));
        for _ in 0..MEMBER_COUNT {
            accumulator.push(AlbumMemberRef::from_cache(&record));
        }
        assert_eq!(accumulator.item_count, MEMBER_COUNT);
        assert_eq!(accumulator.item_size, u64::MAX);

        let album_id = ArrayString::<64>::from("constant-album").unwrap();
        let AbstractData::Album(album) = Album::new(album_id, None).into_abstract_data() else {
            unreachable!()
        };
        let aggregate = accumulator.finish(album, 456);
        assert_eq!(aggregate.metadata.item_count, MEMBER_COUNT);
        assert_eq!(aggregate.metadata.cover, Some(record.id));
        assert_eq!(aggregate.object.thumbhash, Some(vec![1, 2, 3]));
        assert_eq!(aggregate.object.cache_version, 9);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    #[ignore = "local 100k-member album override microbenchmark"]
    #[allow(clippy::too_many_lines)]
    fn benchmark_streaming_album_override_aggregate() {
        use std::hint::black_box;
        use std::time::{Duration, Instant};

        const MEMBER_COUNT: usize = 100_000;

        fn median(mut samples: Vec<Duration>) -> Duration {
            samples.sort_unstable();
            samples[samples.len() / 2]
        }

        fn measure<T>(mut operation: impl FnMut() -> T) -> Duration {
            for _ in 0..2 {
                black_box(operation());
            }
            median(
                (0..9)
                    .map(|_| {
                        let started = Instant::now();
                        black_box(operation());
                        started.elapsed()
                    })
                    .collect(),
            )
        }

        let album_id = ArrayString::<64>::from("album-aggregate-benchmark").unwrap();
        let AbstractData::Album(mut album) = Album::new(album_id, None).into_abstract_data() else {
            unreachable!()
        };
        let mut state = TreeState::default();
        let mut ordinals = Vec::with_capacity(MEMBER_COUNT);
        let mut override_slot = SlotRef::new(0, 0);
        let mut override_id = ArrayString::<64>::new();
        for index in 0..MEMBER_COUNT {
            let id =
                ArrayString::<64>::from(format!("album-bench-member-{index:06}").as_str()).unwrap();
            let mut record = cache_record(id.as_str(), i64::try_from(index % 4096).unwrap());
            record.size = u64::try_from(index).unwrap() + 1;
            record.cache_version = u32::try_from(index).unwrap();
            let slot_ref = state.arena.allocate(record);
            if index == MEMBER_COUNT / 2 {
                override_slot = slot_ref;
                override_id = id;
                album.metadata.cover = Some(id);
            }
            ordinals.push(slot_ref.index());
        }
        state.query.albums.insert(
            album_id,
            OrdinalSet::from_ordinals(ordinals, state.arena.capacity()),
        );
        state.albums.insert(album_id, album);

        let mut metadata = crate::public::structure::image::ImageMetadata::new(
            override_id,
            999_999,
            10,
            20,
            "jpg".to_owned(),
        );
        metadata.albums.insert(album_id);
        let mut object = ObjectSchema::new(override_id, ObjectType::Image);
        object.thumbhash = Some(vec![9, 8, 7]);
        object.cache_version = 42;
        let override_data = AbstractData::Image(crate::public::structure::image::ImageCombined {
            object,
            metadata,
        });
        let changed_at = crate::public::structure::object::next_mutation_timestamp();

        let expected = naive_album_aggregate_with_override(
            &state,
            album_id,
            override_slot,
            &override_data,
            changed_at,
        )
        .unwrap();
        let actual = state
            .album_aggregate_with_override(album_id, override_slot, &override_data, changed_at)
            .unwrap();
        assert_album_eq(&actual, &expected);

        let legacy = measure(|| {
            naive_album_aggregate_with_override(
                black_box(&state),
                album_id,
                override_slot,
                black_box(&override_data),
                changed_at,
            )
            .unwrap()
        });
        let streaming = measure(|| {
            state
                .album_aggregate_with_override(
                    album_id,
                    override_slot,
                    black_box(&override_data),
                    changed_at,
                )
                .unwrap()
        });
        eprintln!(
            "album override legacy={legacy:?} streaming={streaming:?} speedup={:.2}x \
             legacy_scratch={} streaming_scratch={}",
            legacy.as_secs_f64() / streaming.as_secs_f64(),
            MEMBER_COUNT * std::mem::size_of::<CacheRecord>(),
            std::mem::size_of::<AlbumAccumulator<'static>>()
        );
        assert!(streaming < legacy);
        assert!(std::mem::size_of::<AlbumAccumulator<'static>>() <= 64 * 1024);
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
    fn ordinal_set_intersects_handles_sparse_dense_and_word_boundaries() {
        let sparse_left = OrdinalSet::Sparse(vec![1, 63, 129]);
        let sparse_overlap = OrdinalSet::Sparse(vec![2, 63, 130]);
        let sparse_disjoint = OrdinalSet::Sparse(vec![0, 65, 128]);
        assert!(sparse_left.intersects(&sparse_overlap));
        assert!(!sparse_left.intersects(&sparse_disjoint));

        let mut dense_left = DenseBitmap::default();
        for ordinal in [1, 64, 127, 192] {
            dense_left.set(ordinal, true);
        }
        let dense_left = OrdinalSet::Dense(dense_left);
        let mut dense_overlap = DenseBitmap::default();
        for ordinal in [0, 64, 128] {
            dense_overlap.set(ordinal, true);
        }
        let dense_overlap = OrdinalSet::Dense(dense_overlap);
        let mut dense_disjoint = DenseBitmap::default();
        for ordinal in [2, 63, 65, 191] {
            dense_disjoint.set(ordinal, true);
        }
        let dense_disjoint = OrdinalSet::Dense(dense_disjoint);

        assert!(dense_left.intersects(&dense_overlap));
        assert!(!dense_left.intersects(&dense_disjoint));
        assert!(sparse_left.intersects(&dense_left));
        assert!(dense_left.intersects(&sparse_left));
        assert!(!sparse_disjoint.intersects(&dense_left));
        assert!(!dense_left.intersects(&sparse_disjoint));
        assert!(!OrdinalSet::default().intersects(&dense_left));
        assert!(!dense_left.intersects(&OrdinalSet::default()));
    }

    #[test]
    fn concrete_set_iterators_are_exact_fused_and_preserve_overrides() {
        fn assert_exact_fused<I: ExactSizeIterator + FusedIterator>(_iterator: &I) {}

        for ordinals in [
            OrdinalSet::Sparse(vec![1, 63, 64, 130]),
            OrdinalSet::Dense({
                let mut bitmap = DenseBitmap::default();
                for ordinal in [1, 63, 64, 130] {
                    bitmap.set(ordinal, true);
                }
                bitmap
            }),
        ] {
            let mut iterator = ordinals.iter();
            assert_exact_fused(&iterator);
            assert_eq!(iterator.size_hint(), (4, Some(4)));
            assert_eq!(iterator.next(), Some(1));
            assert_eq!(iterator.len(), 3);
            assert_eq!(iterator.collect::<Vec<_>>(), vec![63, 64, 130]);
        }

        let common = TargetSet::from_unique_slot_refs(
            [0, 1, 63, 64, 130].map(|ordinal| SlotRef::new(ordinal, 1)),
            192,
        );
        assert!(matches!(common.iter(), TargetSetIter::GenerationOne(_)));
        assert_eq!(
            common.iter().collect::<Vec<_>>(),
            [0, 1, 63, 64, 130].map(|ordinal| SlotRef::new(ordinal, 1))
        );

        let overrides = TargetSet::from_unique_slot_refs(
            [
                SlotRef::new(0, 2),
                SlotRef::new(1, 1),
                SlotRef::new(63, 3),
                SlotRef::new(64, 1),
                SlotRef::new(130, 4),
            ],
            192,
        );
        let mut iterator = overrides.iter();
        assert_exact_fused(&iterator);
        assert_eq!(iterator.size_hint(), (5, Some(5)));
        assert_eq!(
            iterator.by_ref().collect::<Vec<_>>(),
            vec![
                SlotRef::new(0, 2),
                SlotRef::new(1, 1),
                SlotRef::new(63, 3),
                SlotRef::new(64, 1),
                SlotRef::new(130, 4),
            ]
        );
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.next(), None);
        assert_eq!(iterator.size_hint(), (0, Some(0)));
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
    #[ignore = "local 1M TargetSet/OrdinalSet microbenchmark"]
    #[allow(clippy::too_many_lines)]
    fn benchmark_specialized_set_iteration_and_intersections() {
        use std::hint::black_box;
        use std::time::{Duration, Instant};

        const ITEM_COUNT: usize = 1_000_000;

        fn median(mut samples: Vec<Duration>) -> Duration {
            samples.sort_unstable();
            samples[samples.len() / 2]
        }

        fn measure<T>(mut operation: impl FnMut() -> T) -> Duration {
            for _ in 0..2 {
                black_box(operation());
            }
            median(
                (0..9)
                    .map(|_| {
                        let started = Instant::now();
                        black_box(operation());
                        started.elapsed()
                    })
                    .collect(),
            )
        }

        fn legacy_target_checksum(targets: &TargetSet) -> u64 {
            let ordinals: Box<dyn Iterator<Item = u32> + '_> = match &targets.ordinals {
                OrdinalSet::Sparse(items) => Box::new(items.iter().copied()),
                OrdinalSet::Dense(bitmap) => Box::new(bitmap.iter()),
            };
            ordinals.fold(0_u64, |checksum, ordinal| {
                let generation = targets
                    .generation_overrides
                    .binary_search_by_key(&ordinal, |(index, _)| *index)
                    .ok()
                    .map_or(1, |index| targets.generation_overrides[index].1);
                checksum.wrapping_add(SlotRef::new(ordinal, generation).raw())
            })
        }

        fn specialized_target_checksum(targets: &TargetSet) -> u64 {
            targets.iter().fold(0_u64, |checksum, slot_ref| {
                checksum.wrapping_add(slot_ref.raw())
            })
        }

        fn legacy_intersects(left: &OrdinalSet, right: &OrdinalSet) -> bool {
            let ordinals: Box<dyn Iterator<Item = u32> + '_> = match left {
                OrdinalSet::Sparse(items) => Box::new(items.iter().copied()),
                OrdinalSet::Dense(bitmap) => Box::new(bitmap.iter()),
            };
            ordinals.into_iter().any(|ordinal| right.contains(ordinal))
        }

        let mut all_words = vec![u64::MAX; ITEM_COUNT.div_ceil(64)];
        let trailing = ITEM_COUNT % 64;
        if trailing != 0 {
            *all_words.last_mut().unwrap() = (1_u64 << trailing) - 1;
        }
        let generation_one = TargetSet::from_dense_parts(all_words.clone(), Vec::new());
        let with_overrides = TargetSet::from_dense_parts(
            all_words,
            (0..ITEM_COUNT)
                .step_by(997)
                .map(|ordinal| (u32::try_from(ordinal).unwrap(), 2))
                .collect(),
        );
        assert_eq!(
            legacy_target_checksum(&generation_one),
            specialized_target_checksum(&generation_one)
        );
        assert_eq!(
            legacy_target_checksum(&with_overrides),
            specialized_target_checksum(&with_overrides)
        );

        let legacy_generation_one =
            measure(|| black_box(legacy_target_checksum(black_box(&generation_one))));
        let specialized_generation_one =
            measure(|| black_box(specialized_target_checksum(black_box(&generation_one))));
        let legacy_overrides =
            measure(|| black_box(legacy_target_checksum(black_box(&with_overrides))));
        let specialized_overrides =
            measure(|| black_box(specialized_target_checksum(black_box(&with_overrides))));

        let dense_even = OrdinalSet::Dense(DenseBitmap::from_words(vec![
            0x5555_5555_5555_5555;
            ITEM_COUNT.div_ceil(64)
        ]));
        let dense_odd = OrdinalSet::Dense(DenseBitmap::from_words(vec![
            0xAAAA_AAAA_AAAA_AAAA;
            ITEM_COUNT.div_ceil(64)
        ]));
        assert!(!legacy_intersects(&dense_even, &dense_odd));
        assert!(!dense_even.intersects(&dense_odd));
        let legacy_dense_intersects = measure(|| {
            black_box(legacy_intersects(
                black_box(&dense_even),
                black_box(&dense_odd),
            ))
        });
        let specialized_dense_intersects =
            measure(|| black_box(dense_even.intersects(black_box(&dense_odd))));

        let sparse_left = OrdinalSet::Sparse(
            (0..30_000)
                .map(|index| u32::try_from(index * 32).unwrap())
                .collect(),
        );
        let sparse_right = OrdinalSet::Sparse(
            (0..30_000)
                .map(|index| u32::try_from(index * 32 + 1).unwrap())
                .collect(),
        );
        assert!(!legacy_intersects(&sparse_left, &sparse_right));
        assert!(!sparse_left.intersects(&sparse_right));
        let legacy_sparse_intersects =
            measure(|| black_box(legacy_intersects(&sparse_left, &sparse_right)));
        let specialized_sparse_intersects =
            measure(|| black_box(sparse_left.intersects(&sparse_right)));

        let mixed_dense = OrdinalSet::Dense(DenseBitmap::from_words(vec![
            0xAAAA_AAAA_AAAA_AAAA;
            ITEM_COUNT.div_ceil(64)
        ]));
        assert!(!legacy_intersects(&sparse_left, &mixed_dense));
        assert!(!sparse_left.intersects(&mixed_dense));
        let legacy_mixed_intersects =
            measure(|| black_box(legacy_intersects(&sparse_left, &mixed_dense)));
        let specialized_mixed_intersects =
            measure(|| black_box(sparse_left.intersects(&mixed_dense)));

        eprintln!(
            "sets gen1 legacy={legacy_generation_one:?} specialized={specialized_generation_one:?}; \
             overrides legacy={legacy_overrides:?} specialized={specialized_overrides:?}; \
             dense legacy={legacy_dense_intersects:?} specialized={specialized_dense_intersects:?}; \
             sparse legacy={legacy_sparse_intersects:?} specialized={specialized_sparse_intersects:?}; \
             mixed legacy={legacy_mixed_intersects:?} specialized={specialized_mixed_intersects:?}"
        );
        assert!(specialized_generation_one < legacy_generation_one);
        assert!(specialized_overrides < legacy_overrides);
        assert!(specialized_dense_intersects.saturating_mul(2) <= legacy_dense_intersects);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn structural_epoch_changes_only_for_structural_tree_mutations() {
        let mut records = (0..4)
            .map(|index| AbstractData::generate_performance_data(index, 19))
            .collect::<Vec<_>>();
        let mut state = TreeState::from_records(records.iter().cloned());
        let initial_epoch = state.structural_epoch();
        let identity_epoch = state.identity_epoch();
        let initial_selection_epoch = state.selection_epoch();
        let targets = OrdinalSet::from_ordinals([0, 1], state.arena.capacity());
        state.query.edit_flags(
            &targets,
            FlagPatch {
                favorite: Some(true),
                ..FlagPatch::default()
            },
        );
        assert_eq!(state.structural_epoch(), initial_epoch);
        assert_eq!(state.identity_epoch(), identity_epoch);
        assert_eq!(state.selection_epoch(), initial_selection_epoch);

        let rekeyed = state.find(records[0].hash().as_str()).unwrap();
        records[0].alias_mut().unwrap()[0].modified += 1;
        state.replace_static(rekeyed, &records[0]).unwrap();
        let rekeyed_epoch = state.structural_epoch();
        assert_ne!(rekeyed_epoch, initial_epoch);
        assert_eq!(state.identity_epoch(), identity_epoch);
        assert_eq!(state.selection_epoch(), initial_selection_epoch);

        let inserted = state.insert(&AbstractData::generate_performance_data(100, 19));
        let inserted_epoch = state.structural_epoch();
        assert_ne!(inserted_epoch, rekeyed_epoch);
        assert_eq!(state.identity_epoch(), identity_epoch);
        assert_eq!(state.selection_epoch(), initial_selection_epoch);
        state.remove_targets(&TargetSet::from_slot_refs(
            [inserted],
            state.arena.capacity(),
        ));
        assert_ne!(state.structural_epoch(), inserted_epoch);
        assert_eq!(state.identity_epoch(), identity_epoch);
        assert_ne!(state.selection_epoch(), initial_selection_epoch);
        assert_ne!(TreeState::default().identity_epoch(), identity_epoch);
    }

    #[test]
    fn id_index_verifies_full_id_inside_collision_bucket() {
        let mut arena = RecordArena::default();
        let first = arena.allocate(cache_record("first", 1));
        let second = arena.allocate(cache_record("second", 1));
        let mut index = IdIndex::default();
        let fingerprint = IdIndex::fingerprint("second");
        index.primary.insert(fingerprint, first.index());
        index
            .collisions
            .insert(fingerprint, vec![first.index(), second.index()]);
        assert_eq!(index.find("second", &arena), Some(second));
        assert_eq!(index.find("missing", &arena), None);

        assert!(index.remove("second", second, &arena));
        assert!(!index.collisions.contains_key(&fingerprint));
        assert_eq!(
            index.primary.get(&fingerprint).copied(),
            Some(first.index())
        );
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
        assert!(index.remove("primary", slot, &arena));
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

    #[test]
    fn id_index_stale_remove_cannot_delete_a_reused_slot() {
        let mut arena = RecordArena::default();
        let stale = arena.allocate(cache_record("same-id", 1));
        let mut index = IdIndex::default();
        index.insert("same-id", stale);
        assert!(arena.remove(stale).is_some());

        let replacement = arena.allocate(cache_record("same-id", 2));
        index.insert("same-id", replacement);
        assert_eq!(stale.index(), replacement.index());
        assert_ne!(stale.generation(), replacement.generation());

        assert!(!index.remove("same-id", stale, &arena));
        assert_eq!(index.find("same-id", &arena), Some(replacement));
        assert!(index.remove("same-id", replacement, &arena));
    }

    #[test]
    fn id_index_remove_verifies_the_full_id() {
        let mut arena = RecordArena::default();
        let slot = arena.allocate(cache_record("actual", 1));
        let mut index = IdIndex::default();
        let wrong_fingerprint = IdIndex::fingerprint("wrong");
        index.primary.insert(wrong_fingerprint, slot.index());

        assert!(!index.remove("wrong", slot, &arena));
        assert_eq!(
            index.primary.get(&wrong_fingerprint).copied(),
            Some(slot.index())
        );
    }

    #[test]
    fn canonical_id_fingerprint_uses_hex_prefix_and_noncanonical_falls_back() {
        let canonical = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            canonical_hex_fingerprint(canonical),
            Some(0x0123_4567_89ab_cdef)
        );
        assert_eq!(IdIndex::fingerprint(canonical), 0x0123_4567_89ab_cdef);

        for noncanonical in [
            "short-id",
            "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF",
            "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
        ] {
            assert_eq!(canonical_hex_fingerprint(noncanonical), None);
            assert_eq!(
                IdIndex::fingerprint(noncanonical),
                IdIndex::legacy_fingerprint(noncanonical)
            );
        }

        let mut hasher = U64IdentityHasher::default();
        hasher.write_u64(0xDEAD_BEEF_0123_4567);
        assert_eq!(hasher.finish(), 0xDEAD_BEEF_0123_4567);
    }

    #[test]
    fn canonical_prefix_collision_still_verifies_full_id_and_collapses() {
        let first_id = "0123456789abcdef000000000000000000000000000000000000000000000000";
        let second_id = "0123456789abcdef111111111111111111111111111111111111111111111111";
        assert_eq!(
            IdIndex::fingerprint(first_id),
            IdIndex::fingerprint(second_id)
        );

        let mut arena = RecordArena::default();
        let first = arena.allocate(cache_record(first_id, 1));
        let second = arena.allocate(cache_record(second_id, 2));
        let mut index = IdIndex::default();
        index.insert(first_id, first);
        index.insert(second_id, second);

        assert_eq!(index.find(first_id, &arena), Some(first));
        assert_eq!(index.find(second_id, &arena), Some(second));
        assert_eq!(index.collisions.len(), 1);
        assert!(index.remove(second_id, second, &arena));
        assert!(index.collisions.is_empty());
        assert_eq!(index.find(first_id, &arena), Some(first));
        assert_eq!(index.find(second_id, &arena), None);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    #[ignore = "local 1M canonical ID fingerprint microbenchmark"]
    fn benchmark_million_canonical_id_fingerprints() {
        use std::hint::black_box;
        use std::time::{Duration, Instant};

        const ITEM_COUNT: usize = 1_000_000;

        fn median(mut samples: Vec<Duration>) -> Duration {
            samples.sort_unstable();
            samples[samples.len() / 2]
        }

        fn measure<T>(mut operation: impl FnMut() -> T) -> Duration {
            for _ in 0..2 {
                black_box(operation());
            }
            median(
                (0..9)
                    .map(|_| {
                        let started = Instant::now();
                        black_box(operation());
                        started.elapsed()
                    })
                    .collect(),
            )
        }

        let ids = (0..4_096_u64)
            .map(|value| {
                let mixed = value.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                format!("{value:016x}{mixed:016x}{value:016x}{mixed:016x}")
            })
            .collect::<Vec<_>>();
        assert!(ids.iter().all(|id| id.len() == 64));

        let legacy = measure(|| {
            (0..ITEM_COUNT).fold(0_u64, |checksum, index| {
                checksum.wrapping_add(IdIndex::legacy_fingerprint(black_box(
                    &ids[index % ids.len()],
                )))
            })
        });
        let optimized = measure(|| {
            (0..ITEM_COUNT).fold(0_u64, |checksum, index| {
                checksum.wrapping_add(IdIndex::fingerprint(black_box(&ids[index % ids.len()])))
            })
        });
        eprintln!(
            "canonical ID fingerprint legacy={legacy:?} optimized={optimized:?} speedup={:.2}x",
            legacy.as_secs_f64() / optimized.as_secs_f64()
        );
        assert!(optimized < legacy);
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

    #[cfg(feature = "performance-test")]
    #[test]
    fn owned_preallocated_rebuild_matches_legacy_borrowed_results() {
        let mut records = (0..512)
            .map(|index| AbstractData::generate_performance_data(index, 20_260_718))
            .collect::<Vec<_>>();
        records.push(
            Album::new(
                ArrayString::<64>::from("owned-rebuild-album").unwrap(),
                Some("Owned rebuild".to_owned()),
            )
            .into_abstract_data(),
        );

        let legacy = TreeState::from_records_legacy_owned(records.clone());
        let optimized = TreeState::from_records(records);

        let ordered = |state: &TreeState| {
            state
                .order
                .iter()
                .map(|slot_ref| {
                    let record = state.get(*slot_ref).unwrap();
                    (
                        record.id,
                        record.object_type,
                        record.timestamp,
                        slot_ref.generation(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(ordered(&optimized), ordered(&legacy));
        assert!(
            optimized
                .order
                .iter()
                .all(|slot_ref| slot_ref.generation() == 1)
        );
        assert!(
            legacy
                .order
                .iter()
                .all(|slot_ref| slot_ref.generation() == 1)
        );

        for expression in [
            Expression::Favorite(true),
            Expression::Archived(true),
            Expression::Trashed(true),
            Expression::Ext("JPG".to_owned()),
            Expression::Any("perf-tag-3".to_owned()),
            Expression::ExtType("video".to_owned()),
        ] {
            let matching_ids = |state: &TreeState| {
                state
                    .order
                    .iter()
                    .filter(|slot_ref| state.matches(**slot_ref, &expression, None))
                    .map(|slot_ref| state.get(*slot_ref).unwrap().id)
                    .collect::<Vec<_>>()
            };
            assert_eq!(matching_ids(&optimized), matching_ids(&legacy));
        }

        let album_id = ArrayString::<64>::from("owned-rebuild-album").unwrap();
        assert_eq!(
            serde_json::to_value(optimized.albums.get(&album_id)).unwrap(),
            serde_json::to_value(legacy.albums.get(&album_id)).unwrap()
        );
        assert_eq!(optimized.memory_usage(), legacy.memory_usage());
    }

    #[cfg(feature = "performance-test")]
    #[test]
    fn rebuild_preallocation_matches_natural_capacity_classes() {
        for record_count in [0_usize, 1, 3, 4, 5, 127, 128, 129] {
            let records = (0..record_count)
                .map(|index| AbstractData::generate_performance_data(index as u64, 73))
                .collect::<Vec<_>>();
            let legacy = TreeState::from_records_legacy_owned(records.clone());
            let optimized = TreeState::from_records(records);
            assert_eq!(
                optimized.arena.slots.capacity(),
                legacy.arena.slots.capacity(),
                "arena capacity for {record_count} records"
            );
            assert_eq!(
                optimized.order.capacity(),
                legacy.order.capacity(),
                "order capacity for {record_count} records"
            );
            assert_eq!(
                optimized.id_index.primary.capacity(),
                legacy.id_index.primary.capacity(),
                "ID index capacity for {record_count} records"
            );
            assert_eq!(
                optimized.query.tags.membership_count.values.capacity(),
                legacy.query.tags.membership_count.values.capacity(),
                "tag count capacity for {record_count} records"
            );
            assert_eq!(
                optimized.query.album_membership_count.values.capacity(),
                legacy.query.album_membership_count.values.capacity(),
                "album count capacity for {record_count} records"
            );
        }

        let initial = (0..128)
            .map(|index| AbstractData::generate_performance_data(index, 73))
            .collect::<Vec<_>>();
        let mut state = TreeState::from_records(initial);
        let arena_capacity = state.arena.slots.capacity();
        let order_capacity = state.order.capacity();
        state.insert(&AbstractData::generate_performance_data(128, 73));
        assert!(state.arena.slots.capacity() <= arena_capacity.saturating_mul(2));
        assert!(state.order.capacity() <= order_capacity.saturating_mul(2));
        assert_eq!(state.len(), 129);
    }

    #[cfg(feature = "performance-test")]
    #[test]
    #[ignore = "local TreeState owned rebuild microbenchmark"]
    fn owned_preallocated_rebuild_beats_legacy_borrowed_reference() -> anyhow::Result<()> {
        use std::hint::black_box;
        use std::time::Duration;

        use crate::storage::DataStore;

        const RECORDS: usize = 100_000;
        const SAMPLES: usize = 9;

        fn median(mut values: Vec<Duration>) -> Duration {
            values.sort_unstable();
            values[values.len() / 2]
        }

        let fixture = (0..RECORDS)
            .map(|index| AbstractData::generate_performance_data(index as u64, 20_260_718))
            .collect::<Vec<_>>();
        let directory = tempfile::tempdir()?;
        let store = DataStore::initialize_empty(&directory.path().join("rebuild.redb"))?;
        store.write(|writer| {
            for record in &fixture {
                writer.insert(record)?;
            }
            Ok::<(), anyhow::Error>(())
        })?;
        let reader = store.reader()?;

        let legacy_rebuild = || -> anyhow::Result<TreeState> {
            let records = reader
                .iter()?
                .map(|entry| entry.map(|(_, value)| value.into_value()));
            TreeState::try_from_records_legacy_owned(records)
        };
        let optimized_rebuild = || -> anyhow::Result<TreeState> {
            let records = reader
                .values()?
                .map(|entry| entry.map(crate::storage::store::RecordValue::into_value));
            TreeState::try_from_records_with_capacity(records, RECORDS)
        };

        black_box(legacy_rebuild()?);
        black_box(optimized_rebuild()?);

        let mut legacy_samples = Vec::with_capacity(SAMPLES);
        let mut optimized_samples = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            if sample.is_multiple_of(2) {
                let started = Instant::now();
                black_box(legacy_rebuild()?);
                legacy_samples.push(started.elapsed());
                let started = Instant::now();
                black_box(optimized_rebuild()?);
                optimized_samples.push(started.elapsed());
            } else {
                let started = Instant::now();
                black_box(optimized_rebuild()?);
                optimized_samples.push(started.elapsed());
                let started = Instant::now();
                black_box(legacy_rebuild()?);
                legacy_samples.push(started.elapsed());
            }
        }

        let legacy = median(legacy_samples);
        let optimized = median(optimized_samples);
        eprintln!("TreeState rebuild microbench: legacy={legacy:?} optimized={optimized:?}");
        assert!(
            optimized <= legacy,
            "owned preallocated rebuild regressed against the legacy reference"
        );
        Ok(())
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
                    let position = usize::try_from(
                        random % u64::try_from(state.order.len()).expect("order length fits u64"),
                    )
                    .expect("random position fits usize");
                    let slot_ref = state.order[position];
                    let id = state.get(slot_ref).unwrap().id;
                    let targets = TargetSet::from_slot_refs([slot_ref], state.arena.capacity());
                    state.remove_targets(&targets);
                    records.remove(&id);
                }
                2 if !state.order.is_empty() => {
                    let position = usize::try_from(
                        random % u64::try_from(state.order.len()).expect("order length fits u64"),
                    )
                    .expect("random position fits usize");
                    let id = state.get(state.order[position]).unwrap().id;
                    let record = records.get_mut(&id).unwrap();
                    if let Some(alias) = record.alias_mut().and_then(|aliases| aliases.first_mut())
                    {
                        alias.modified = alias.modified.saturating_add(10_000 + step);
                        alias.scan_time = alias.modified;
                    }
                    state.apply_batch(std::slice::from_ref(record), &HashSet::new());
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
