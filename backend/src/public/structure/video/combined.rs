use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::metadata::VideoMetadata;
use crate::public::structure::object::ObjectSchema;

/// Combined Video data with Object and Metadata
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct VideoCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: VideoMetadata,
}
