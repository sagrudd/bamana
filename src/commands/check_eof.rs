use std::path::PathBuf;

use serde::Serialize;

use crate::{
    error::AppError,
    formats::{
        bgzf,
        probe::{ContainerKind, DetectedFormat, probe_path},
    },
};

#[derive(Debug)]
pub struct CheckEofRequest {
    pub bam: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct CheckEofResponse {
    pub detected_format: DetectedFormat,
    pub bgzf_eof_present: bool,
    pub complete: bool,
    pub semantic_note: String,
}

pub fn run(request: CheckEofRequest) -> Result<CheckEofResponse, AppError> {
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

    let bgzf_eof_present = bgzf::has_bgzf_eof(&request.bam)?;
    if !bgzf_eof_present {
        return Err(AppError::TruncatedFile {
            path: request.bam,
            detail: "Expected BGZF EOF marker was not found.".to_string(),
        });
    }

    Ok(CheckEofResponse {
        detected_format: DetectedFormat::Bam,
        bgzf_eof_present: true,
        complete: true,
        semantic_note: "EOF marker presence indicates tail completeness only and does not imply full BAM validity.".to_string(),
    })
}
