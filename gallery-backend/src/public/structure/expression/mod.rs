use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum FilterValue {
    Value(String),
    Exists(bool),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum AlbumFilterValue {
    Value(ArrayString<64>),
    Exists(bool),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Expression {
    Or(Vec<Expression>),
    And(Vec<Expression>),
    Not(Box<Expression>),
    Tag(FilterValue),
    ExtType(String),
    Ext(String),
    Model(FilterValue),
    Make(FilterValue),
    Path(String),
    Album(AlbumFilterValue),
    Any(String),
    // Boolean field filters
    Favorite(bool),
    Archived(bool),
    Trashed(bool),
}
