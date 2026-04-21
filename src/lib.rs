//! Bamana native core library.
//!
//! Performance-critical BAM, BGZF, FASTQ, sampling, and forensic paths are
//! expected to live here under Bamana-owned modules. External format crates are
//! compatibility tools, not the architectural center of the execution engine.

pub mod bam;
pub mod bgzf;
pub mod cli;
pub mod commands;
pub mod error;
pub mod fasta;
pub mod fastq;
pub mod forensics;
pub mod formats;
pub mod ingest;
pub mod json;
pub mod sampling;
