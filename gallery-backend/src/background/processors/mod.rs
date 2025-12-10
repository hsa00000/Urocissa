//! Processors module - domain-specific processing logic
//!
//! This module contains the following submodules:
//! - `image`: Image processing logic (thumbnail generation, orientation fix, hash computation, etc.)
//! - `video`: Video processing logic (thumbnail, compression, metadata, etc.)
//! - `metadata`: EXIF and timestamp metadata handling
//! - `setup`: Initialization setup (ffmpeg check, folder creation, logger, etc.)
//! - `transitor`: Data transformation utilities

pub mod image;
pub mod metadata;
pub mod setup;
pub mod transitor;
pub mod video;
