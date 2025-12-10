use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, bitcode::Decode, bitcode::Encode)]
pub struct ReducedData {
    pub hash: ArrayString<64>,
    pub width: u32,
    pub height: u32,
    pub date: u128,
}
