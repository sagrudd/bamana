use std::path::PathBuf;

use crate::{
    bam::header::HeaderPayload,
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
};

#[derive(Debug)]
pub struct HeaderRequest {
    pub bam: PathBuf,
}

pub type HeaderResponse = HeaderPayload;

pub fn run(request: HeaderRequest) -> Result<HeaderResponse, AppError> {
    let probe = probe_path(&request.bam)?;

    if probe.detected_format == DetectedFormat::Unknown {
        return Err(AppError::UnknownFormat { path: request.bam });
    }

    if probe.detected_format != DetectedFormat::Bam {
        return Err(AppError::NotBam {
            path: request.bam,
            detected_format: probe.detected_format,
        });
    }

    if probe.container != ContainerKind::Bgzf {
        return Err(AppError::InvalidBam {
            path: request.bam,
            detail: "Input did not present a BGZF-compatible container header.".to_string(),
        });
    }

    crate::bam::header::parse_bam_header(&request.bam)
}
