use serde::{Deserialize, Serialize};

use super::metadata::AlbumMetadata;
use crate::public::structure::object::ObjectSchema;

/// Combined Album data with Object and Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: AlbumMetadata,
}
