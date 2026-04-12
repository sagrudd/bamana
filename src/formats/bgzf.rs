//! Transitional compatibility shim.
//!
//! The Bamana-native BGZF core now lives at `crate::bgzf`. This module remains
//! only to keep older internal paths stable during migration. Do not expand new
//! hot-path code under `crate::formats::bgzf`.

pub use crate::bgzf::*;
