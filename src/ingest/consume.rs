use clap::ValueEnum;
use serde::Serialize;

use crate::formats::probe::DetectedFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ConsumeMode {
    Alignment,
    Unmapped,
    MixedAllow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ConsumeSortOrder {
    None,
    Coordinate,
    Queryname,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConsumePlatform {
    Ont,
    Illumina,
    Pacbio,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSemanticClass {
    Alignment,
    RawRead,
    Unsupported,
}

pub fn classify_input_format(format: DetectedFormat) -> InputSemanticClass {
    match format {
        DetectedFormat::Bam | DetectedFormat::Sam => InputSemanticClass::Alignment,
        DetectedFormat::Fastq | DetectedFormat::FastqGz => InputSemanticClass::RawRead,
        _ => InputSemanticClass::Unsupported,
    }
}

pub fn mapped_state_for_mode(mode: ConsumeMode) -> &'static str {
    match mode {
        ConsumeMode::Alignment => "mapped_or_mixed",
        ConsumeMode::Unmapped => "unmapped",
        ConsumeMode::MixedAllow => "indeterminate",
    }
}

pub fn header_strategy_for_mode(mode: ConsumeMode) -> &'static str {
    match mode {
        ConsumeMode::Alignment => "first_compatible_alignment_header",
        ConsumeMode::Unmapped => "synthetic_unmapped_header",
        ConsumeMode::MixedAllow => "mixed_ingest_policy",
    }
}
