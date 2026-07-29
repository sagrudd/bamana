//! Bamana-native BAM core.
//!
//! Production BAM hot paths should be implemented here rather than delegated to
//! general-purpose external format crates.

pub mod adenine_modifications;
pub mod annotate_rg;
pub mod checksum;
pub mod fastq;
pub mod header;
pub mod index;
pub mod merge;
pub mod modifications;
pub mod reader;
pub mod record;
pub mod records;
pub mod region;
pub mod region_plan;
pub mod region_traversal;
pub mod reheader;
pub mod scan;
pub mod sort;
mod sort_integrity;
pub mod stream_sort;
pub mod summary;
pub mod tags;
pub mod unmap;
pub mod validate;
pub mod write;
