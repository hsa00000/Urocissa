use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};

pub mod generator;
pub mod privacy;
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Expression {
    Or(Vec<Expression>),
    And(Vec<Expression>),
    Not(Box<Expression>),
    Tag(String),
    ExtType(String),
    Ext(String),
    Model(String),
    Make(String),
    Path(String),
    Album(ArrayString<64>),
    Any(String),
    Favorite(bool),
    Archived(bool),
    Trashed(bool),
}
