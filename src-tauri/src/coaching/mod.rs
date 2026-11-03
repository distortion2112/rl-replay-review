//! Compares a player's games to rank benchmarks and produces an evidence pack:
//! the structured, versioned facts that coaching text (#11) is written from.
//! The engine decides what is true; nothing downstream adds facts.

pub mod engine;
pub mod models;

pub use engine::{generate_report, percentile};
pub use models::*;
