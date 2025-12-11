//! Workflow module - orchestrates media processing pipelines
//!
//! Structure:
//! - `types`: Core data structures (ProcessingGuard)
//! - `processors`: Domain-specific processing logic (image, video, metadata, file, setup, transitor)
//! - `tasks`: Task executors organized by stage (actor, batcher)
//! - `flows`: High-level business flows (index_workflow)

pub mod actors;
pub mod batchers;
pub mod flows;
pub mod processors;
pub mod types;
