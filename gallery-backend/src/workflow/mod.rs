//! Workflow module - orchestrates media processing pipelines
//!
//! Structure:
//! - `types`: Core data structures (ProcessingGuard)
//! - `processors`: Domain-specific processing logic (image, video, metadata, file, setup, transitor)
//! - `tasks`: Task executors organized by stage (actor, batcher)
//! - `flows`: High-level business flows (index_for_watch)

pub mod flows;
pub mod processors;
pub mod tasks;
pub mod types;

// Re-exports for public API
pub use flows::index_for_watch;
