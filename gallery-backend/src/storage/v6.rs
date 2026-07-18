//! Frozen V6 bitcode record schema.
//!
//! Do not change the field order, field types, enum order, or bitcode version
//! used by these structs. A durable shape change requires a new database
//! version and an explicit migration. Runtime/domain structs intentionally do
//! not implement the on-disk format.

#![allow(clippy::struct_excessive_bools)]

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Result, anyhow};
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use redb::{TypeName, Value};

use super::legacy_v5::{
    LegacyAbstractData, LegacyAlbumCombined, LegacyAlbumMetadata, LegacyFileModify,
    LegacyImageCombined, LegacyImageMetadata, LegacyObjectSchema, LegacyObjectType, LegacyShare,
    LegacyVideoCombined, LegacyVideoMetadata,
};
use crate::public::structure::{
    abstract_data::AbstractData,
    album::{combined::AlbumCombined, metadata::AlbumMetadata, share::Share},
    common::FileModify,
    image::{combined::ImageCombined, metadata::ImageMetadata},
    object::{ObjectSchema, ObjectType},
    video::{combined::VideoCombined, metadata::VideoMetadata},
};

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum V6AbstractData {
    Image(V6ImageCombined),
    Video(V6VideoCombined),
    Album(V6AlbumCombined),
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct V6ImageCombined {
    pub object: V6ObjectSchema,
    pub metadata: V6ImageMetadata,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct V6VideoCombined {
    pub object: V6ObjectSchema,
    pub metadata: V6VideoMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct V6AlbumCombined {
    pub object: V6ObjectSchema,
    pub metadata: V6AlbumMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct V6ObjectSchema {
    pub id: ArrayString<64>,
    pub obj_type: V6ObjectType,
    pub pending: bool,
    pub thumbhash: Option<Vec<u8>>,
    /// Reserved from the first V6 writer. It is mapped to the runtime field by
    /// the later thumbnail-cache versioning commit.
    pub cache_version: u32,
    pub description: Option<String>,
    pub tags: HashSet<String>,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub is_trashed: bool,
    pub update_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum V6ObjectType {
    Image,
    Video,
    Album,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct V6ImageMetadata {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub phash: Option<Vec<u8>>,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
    pub alias: Vec<V6FileModify>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct V6VideoMetadata {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub duration: f64,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
    pub alias: Vec<V6FileModify>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct V6AlbumMetadata {
    pub id: ArrayString<64>,
    pub title: Option<String>,
    pub created_time: i64,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub last_modified_time: i64,
    pub cover: Option<ArrayString<64>>,
    pub item_count: usize,
    pub item_size: u64,
    pub share_list: HashMap<ArrayString<64>, V6Share>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct V6FileModify {
    pub file: String,
    pub modified: i64,
    pub scan_time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct V6Share {
    pub url: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: i64,
}

impl Value for V6AbstractData {
    type SelfType<'a> = Self;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bitcode::decode(data).expect("failed to decode V6 AbstractData")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
        bitcode::encode(value)
    }

    fn type_name() -> TypeName {
        TypeName::new("AbstractDataV6")
    }
}

fn v6_object_type(value: ObjectType) -> V6ObjectType {
    match value {
        ObjectType::Image => V6ObjectType::Image,
        ObjectType::Video => V6ObjectType::Video,
        ObjectType::Album => V6ObjectType::Album,
    }
}

fn domain_object_type(value: V6ObjectType) -> ObjectType {
    match value {
        V6ObjectType::Image => ObjectType::Image,
        V6ObjectType::Video => ObjectType::Video,
        V6ObjectType::Album => ObjectType::Album,
    }
}

fn v6_file_modify(value: &FileModify) -> V6FileModify {
    V6FileModify {
        file: value.file.clone(),
        modified: value.modified,
        scan_time: value.scan_time,
    }
}

fn domain_file_modify(value: V6FileModify) -> FileModify {
    FileModify {
        file: value.file,
        modified: value.modified,
        scan_time: value.scan_time,
    }
}

fn v6_share(value: &Share) -> V6Share {
    V6Share {
        url: value.url,
        description: value.description.clone(),
        password: value.password.clone(),
        show_metadata: value.show_metadata,
        show_download: value.show_download,
        show_upload: value.show_upload,
        exp: value.exp,
    }
}

fn domain_share(value: V6Share) -> Share {
    Share {
        url: value.url,
        description: value.description,
        password: value.password,
        show_metadata: value.show_metadata,
        show_download: value.show_download,
        show_upload: value.show_upload,
        exp: value.exp,
    }
}

impl V6ObjectSchema {
    fn from_domain(value: &ObjectSchema) -> Self {
        Self {
            id: value.id,
            obj_type: v6_object_type(value.obj_type),
            pending: value.pending,
            thumbhash: value.thumbhash.clone(),
            cache_version: 0,
            description: value.description.clone(),
            tags: value.tags.clone(),
            is_favorite: value.is_favorite,
            is_archived: value.is_archived,
            is_trashed: value.is_trashed,
            update_at: value.update_at,
        }
    }

    fn into_domain(self) -> ObjectSchema {
        ObjectSchema {
            id: self.id,
            obj_type: domain_object_type(self.obj_type),
            pending: self.pending,
            thumbhash: self.thumbhash,
            description: self.description,
            tags: self.tags,
            is_favorite: self.is_favorite,
            is_archived: self.is_archived,
            is_trashed: self.is_trashed,
            update_at: self.update_at,
        }
    }
}

impl From<&AbstractData> for V6AbstractData {
    fn from(value: &AbstractData) -> Self {
        match value {
            AbstractData::Image(value) => Self::Image(V6ImageCombined {
                object: V6ObjectSchema::from_domain(&value.object),
                metadata: V6ImageMetadata {
                    id: value.metadata.id,
                    size: value.metadata.size,
                    width: value.metadata.width,
                    height: value.metadata.height,
                    ext: value.metadata.ext.clone(),
                    phash: value.metadata.phash.clone(),
                    albums: value.metadata.albums.clone(),
                    exif_vec: value.metadata.exif_vec.clone(),
                    alias: value.metadata.alias.iter().map(v6_file_modify).collect(),
                },
            }),
            AbstractData::Video(value) => Self::Video(V6VideoCombined {
                object: V6ObjectSchema::from_domain(&value.object),
                metadata: V6VideoMetadata {
                    id: value.metadata.id,
                    size: value.metadata.size,
                    width: value.metadata.width,
                    height: value.metadata.height,
                    ext: value.metadata.ext.clone(),
                    duration: value.metadata.duration,
                    albums: value.metadata.albums.clone(),
                    exif_vec: value.metadata.exif_vec.clone(),
                    alias: value.metadata.alias.iter().map(v6_file_modify).collect(),
                },
            }),
            AbstractData::Album(value) => Self::Album(V6AlbumCombined {
                object: V6ObjectSchema::from_domain(&value.object),
                metadata: V6AlbumMetadata {
                    id: value.metadata.id,
                    title: value.metadata.title.clone(),
                    created_time: value.metadata.created_time,
                    start_time: value.metadata.start_time,
                    end_time: value.metadata.end_time,
                    last_modified_time: value.metadata.last_modified_time,
                    cover: value.metadata.cover,
                    item_count: value.metadata.item_count,
                    item_size: value.metadata.item_size,
                    share_list: value
                        .metadata
                        .share_list
                        .iter()
                        .map(|(id, share)| (*id, v6_share(share)))
                        .collect(),
                },
            }),
        }
    }
}

impl V6AbstractData {
    pub fn id(&self) -> &str {
        match self {
            Self::Image(value) => value.object.id.as_str(),
            Self::Video(value) => value.object.id.as_str(),
            Self::Album(value) => value.object.id.as_str(),
        }
    }

    pub fn object_mut(&mut self) -> &mut V6ObjectSchema {
        match self {
            Self::Image(value) => &mut value.object,
            Self::Video(value) => &mut value.object,
            Self::Album(value) => &mut value.object,
        }
    }

    pub fn albums_mut(&mut self) -> Option<&mut HashSet<ArrayString<64>>> {
        match self {
            Self::Image(value) => Some(&mut value.metadata.albums),
            Self::Video(value) => Some(&mut value.metadata.albums),
            Self::Album(_) => None,
        }
    }

    pub fn into_domain(self) -> Result<AbstractData> {
        match self {
            Self::Image(value) => {
                if value.object.obj_type != V6ObjectType::Image {
                    return Err(anyhow!("V6 image record has a non-image object type"));
                }
                Ok(AbstractData::Image(ImageCombined {
                    object: value.object.into_domain(),
                    metadata: ImageMetadata {
                        id: value.metadata.id,
                        size: value.metadata.size,
                        width: value.metadata.width,
                        height: value.metadata.height,
                        ext: value.metadata.ext,
                        phash: value.metadata.phash,
                        albums: value.metadata.albums,
                        exif_vec: value.metadata.exif_vec,
                        alias: value
                            .metadata
                            .alias
                            .into_iter()
                            .map(domain_file_modify)
                            .collect(),
                    },
                }))
            }
            Self::Video(value) => {
                if value.object.obj_type != V6ObjectType::Video {
                    return Err(anyhow!("V6 video record has a non-video object type"));
                }
                Ok(AbstractData::Video(VideoCombined {
                    object: value.object.into_domain(),
                    metadata: VideoMetadata {
                        id: value.metadata.id,
                        size: value.metadata.size,
                        width: value.metadata.width,
                        height: value.metadata.height,
                        ext: value.metadata.ext,
                        duration: value.metadata.duration,
                        albums: value.metadata.albums,
                        exif_vec: value.metadata.exif_vec,
                        alias: value
                            .metadata
                            .alias
                            .into_iter()
                            .map(domain_file_modify)
                            .collect(),
                    },
                }))
            }
            Self::Album(value) => {
                if value.object.obj_type != V6ObjectType::Album {
                    return Err(anyhow!("V6 album record has a non-album object type"));
                }
                Ok(AbstractData::Album(AlbumCombined {
                    object: value.object.into_domain(),
                    metadata: AlbumMetadata {
                        id: value.metadata.id,
                        title: value.metadata.title,
                        created_time: value.metadata.created_time,
                        start_time: value.metadata.start_time,
                        end_time: value.metadata.end_time,
                        last_modified_time: value.metadata.last_modified_time,
                        cover: value.metadata.cover,
                        item_count: value.metadata.item_count,
                        item_size: value.metadata.item_size,
                        share_list: value
                            .metadata
                            .share_list
                            .into_iter()
                            .map(|(id, share)| (id, domain_share(share)))
                            .collect(),
                    },
                }))
            }
        }
    }

    pub fn from_v5(value: LegacyAbstractData) -> Result<Self> {
        match value {
            LegacyAbstractData::Image(LegacyImageCombined { object, metadata }) => {
                if !matches!(object.obj_type, LegacyObjectType::Image) {
                    return Err(anyhow!("V5 image record has a non-image object type"));
                }
                Ok(Self::Image(V6ImageCombined {
                    object: v5_object(object),
                    metadata: v5_image_metadata(metadata),
                }))
            }
            LegacyAbstractData::Video(LegacyVideoCombined { object, metadata }) => {
                if !matches!(object.obj_type, LegacyObjectType::Video) {
                    return Err(anyhow!("V5 video record has a non-video object type"));
                }
                Ok(Self::Video(V6VideoCombined {
                    object: v5_object(object),
                    metadata: v5_video_metadata(metadata),
                }))
            }
            LegacyAbstractData::Album(LegacyAlbumCombined { object, metadata }) => {
                if !matches!(object.obj_type, LegacyObjectType::Album) {
                    return Err(anyhow!("V5 album record has a non-album object type"));
                }
                Ok(Self::Album(V6AlbumCombined {
                    object: v5_object(object),
                    metadata: v5_album_metadata(metadata),
                }))
            }
        }
    }
}

fn v5_object_type(value: LegacyObjectType) -> V6ObjectType {
    match value {
        LegacyObjectType::Image => V6ObjectType::Image,
        LegacyObjectType::Video => V6ObjectType::Video,
        LegacyObjectType::Album => V6ObjectType::Album,
    }
}

fn v5_object(value: LegacyObjectSchema) -> V6ObjectSchema {
    V6ObjectSchema {
        id: value.id,
        obj_type: v5_object_type(value.obj_type),
        pending: value.pending,
        thumbhash: value.thumbhash,
        cache_version: 0,
        description: value.description,
        tags: value.tags,
        is_favorite: value.is_favorite,
        is_archived: value.is_archived,
        is_trashed: value.is_trashed,
        update_at: value.update_at,
    }
}

fn v5_file_modify(value: LegacyFileModify) -> V6FileModify {
    V6FileModify {
        file: value.file,
        modified: value.modified,
        scan_time: value.scan_time,
    }
}

fn v5_share(value: LegacyShare) -> V6Share {
    V6Share {
        url: value.url,
        description: value.description,
        password: value.password,
        show_metadata: value.show_metadata,
        show_download: value.show_download,
        show_upload: value.show_upload,
        exp: value.exp,
    }
}

fn v5_image_metadata(value: LegacyImageMetadata) -> V6ImageMetadata {
    V6ImageMetadata {
        id: value.id,
        size: value.size,
        width: value.width,
        height: value.height,
        ext: value.ext,
        phash: value.phash,
        albums: value.albums,
        exif_vec: value.exif_vec,
        alias: value.alias.into_iter().map(v5_file_modify).collect(),
    }
}

fn v5_video_metadata(value: LegacyVideoMetadata) -> V6VideoMetadata {
    V6VideoMetadata {
        id: value.id,
        size: value.size,
        width: value.width,
        height: value.height,
        ext: value.ext,
        duration: value.duration,
        albums: value.albums,
        exif_vec: value.exif_vec,
        alias: value.alias.into_iter().map(v5_file_modify).collect(),
    }
}

fn v5_album_metadata(value: LegacyAlbumMetadata) -> V6AlbumMetadata {
    V6AlbumMetadata {
        id: value.id,
        title: value.title,
        created_time: value.created_time,
        start_time: value.start_time,
        end_time: value.end_time,
        last_modified_time: value.last_modified_time,
        cover: value.cover,
        item_count: value.item_count,
        item_size: value.item_size,
        share_list: value
            .share_list
            .into_iter()
            .map(|(id, share)| (id, v5_share(share)))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v6_type_name_is_frozen() {
        assert_eq!(V6AbstractData::type_name().name(), "AbstractDataV6");
    }
}
