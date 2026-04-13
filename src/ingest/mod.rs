pub mod consume;
// Transitional compatibility slice backed by noodles for CRAM only.
pub mod cram;
pub mod discovery;
// Compatibility shim; the native FASTQ core now lives at crate::fastq.
pub mod fastq;
pub mod sam;
