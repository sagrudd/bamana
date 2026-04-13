//! Bamana-native BAM core.
//!
//! Production BAM hot paths should be implemented here rather than delegated to
//! general-purpose external format crates.

pub mod annotate_rg;
pub mod checksum;
pub mod header;
pub mod index;
pub mod merge;
pub mod reader;
pub mod records;
pub mod reheader;
pub mod sort;
pub mod summary;
pub mod tags;
pub mod validate;
pub mod write;
