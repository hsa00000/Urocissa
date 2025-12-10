use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct DisplayElement {
    pub display_width: u32,
    pub display_height: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub start: usize,
    pub end: usize,
    pub display_elements: Vec<DisplayElement>,
    pub row_index: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScrollBarData {
    pub year: usize,
    pub month: usize,
    pub index: usize,
}
