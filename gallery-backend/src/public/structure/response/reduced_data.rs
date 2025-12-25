use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Decode, Encode)]
pub struct ReducedData {
    pub hash: ArrayString<64>,
    pub width: u32,
    pub height: u32,
    pub date: u128,
}
