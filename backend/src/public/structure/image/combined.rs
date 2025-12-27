use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::metadata::ImageMetadata;
use crate::public::structure::object::ObjectSchema;

/// Combined Image data with Object and Metadata
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct ImageCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: ImageMetadata,
}
