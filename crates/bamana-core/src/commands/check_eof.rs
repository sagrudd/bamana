use std::path::PathBuf;

use crate::{
    error::BamanaError,
    formats::{
        bgzf,
        probe::{DetectedFormat, probe_path},
    },
    json::schema::CheckEofData,
};

#[derive(Debug)]
pub struct CheckEofRequest {
    pub bam: PathBuf,
}

pub fn run(request: CheckEofRequest) -> Result<CheckEofData, BamanaError> {
    let probe = probe_path(&request.bam)?;
    if probe.detected_format != DetectedFormat::BAM {
        return Err(BamanaError::NotBam {
            path: request.bam,
            detected_format: probe.detected_format,
        });
    }

    let bgzf_eof_present = bgzf::has_canonical_eof_marker(&request.bam)?;
    Ok(CheckEofData {
        detected_format: probe.detected_format,
        bgzf_eof_present,
        complete: bgzf_eof_present,
        semantic_note: "EOF marker presence indicates tail completeness only and does not imply full BAM validity.".to_string(),
    })
}
