//! Transitional compatibility shim.
//!
//! The Bamana-native FASTQ core now lives at `crate::fastq`. This module
//! remains only to keep older internal paths stable during migration. Do not
//! expand new hot-path code under `crate::ingest::fastq`.

pub use crate::fastq::*;
