use std::path::PathBuf;

use crate::{
    error::BamanaError,
    formats::{
        bam,
        probe::{DetectedFormat, probe_path},
    },
    json::schema::HeaderData,
};

#[derive(Debug)]
pub struct HeaderRequest {
    pub bam: PathBuf,
}

pub fn run(request: HeaderRequest) -> Result<HeaderData, BamanaError> {
    let probe = probe_path(&request.bam)?;
    if probe.detected_format != DetectedFormat::BAM || !probe.bam_magic_present {
        return Err(BamanaError::NotBam {
            path: request.bam,
            detected_format: probe.detected_format,
        });
    }

    let header = bam::read_header(&request.bam)?;
    Ok(HeaderData {
        format: DetectedFormat::BAM,
        header,
    })
}
