//! Bitcode codec for disposable query and tree caches.
//!
//! Durable gallery records use the frozen V6 types in `storage::v6`
//! directly. These helpers are intentionally limited to derived caches that
//! may be discarded whenever their in-memory shape changes.

use anyhow::{Context, Result};
use bitcode::{DecodeOwned, Encode};

pub fn encode<T: Encode + ?Sized>(value: &T) -> Vec<u8> {
    bitcode::encode(value)
}

pub fn decode<T: DecodeOwned>(bytes: &[u8]) -> Result<T> {
    bitcode::decode(bytes).context("failed to decode bitcode cache value")
}
