use crate::router::get::get_prefetch::Prefetch;

use crate::public::structure::{
    album::Album, database_struct::database::definition::Database, reduced_data::ReducedData,
    row::Row,
};
use redb::{TypeName, Value};

impl Value for ReducedData {
    type SelfType<'a>
        = Self
    where
        Self: 'a;
    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bitcode::decode::<ReducedData>(data)
            .map_err(|e| {
                error!("Failed to deserialize ReducedData: {:?}", e);
                e
            })
            .unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
        bitcode::encode(value)
    }

    fn type_name() -> TypeName {
        TypeName::new("ReducedData")
    }
}

impl Value for Row {
    type SelfType<'a>
        = Self
    where
        Self: 'a;
    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bitcode::decode::<Self>(data).expect("Failed to deserialize Row")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
        bitcode::encode(value)
    }

    fn type_name() -> TypeName {
        TypeName::new("Row")
    }
}

impl Value for Prefetch {
    type SelfType<'a>
        = Self
    where
        Self: 'a;
    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bitcode::decode::<Self>(data).expect("Failed to deserialize Prefetch")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
        bitcode::encode(value)
    }

    fn type_name() -> TypeName {
        TypeName::new("Prefetch")
    }
}
