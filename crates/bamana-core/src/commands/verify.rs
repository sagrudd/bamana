use std::path::PathBuf;

use crate::{
    error::BamanaError,
    formats::probe::{DetectedFormat, probe_path},
    json::schema::VerifyData,
};

#[derive(Debug)]
pub struct VerifyRequest {
    pub bam: PathBuf,
}

pub fn run(request: VerifyRequest) -> Result<VerifyData, BamanaError> {
    let probe = probe_path(&request.bam)?;
    let data = VerifyData {
        detected_format: probe.detected_format,
        container: probe.container,
        is_bam: probe.detected_format == DetectedFormat::BAM && probe.bam_magic_present,
        shallow_verified: probe.detected_format == DetectedFormat::BAM && probe.bam_magic_present,
        deep_validated: false,
    };

    if probe.detected_format == DetectedFormat::BAM && probe.bam_magic_present {
        return Ok(data);
    }

    if probe.container.is_some() {
        return Err(BamanaError::InvalidBam {
            path: request.bam,
            detail: "BGZF-like container was detected but the BAM magic header was not present in the decompressed payload.".to_string(),
        });
    }

    Err(BamanaError::NotBam {
        path: request.bam,
        detected_format: probe.detected_format,
    })
}
