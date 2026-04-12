use std::path::PathBuf;

use serde::Serialize;

use crate::{
    error::AppError,
    formats::probe::{Confidence, ContainerKind, DetectedFormat, probe_path},
};

#[derive(Debug)]
pub struct VerifyRequest {
    pub bam: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub detected_format: DetectedFormat,
    pub container: ContainerKind,
    pub is_bam: bool,
    pub shallow_verified: bool,
    pub deep_validated: bool,
    pub confidence: Confidence,
    pub checks_performed: Vec<&'static str>,
    pub semantic_note: String,
}

pub fn run(request: VerifyRequest) -> Result<VerifyResponse, AppError> {
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

    if !probe.bam_magic_present {
        return Err(AppError::InvalidBam {
            path: request.bam,
            detail: "The first inflated BGZF member did not begin with BAM\\1 magic.".to_string(),
        });
    }

    Ok(VerifyResponse {
        detected_format: DetectedFormat::Bam,
        container: ContainerKind::Bgzf,
        is_bam: true,
        shallow_verified: true,
        deep_validated: false,
        confidence: probe.confidence,
        checks_performed: vec![
            "opened_input",
            "confirmed_bgzf_container_header",
            "inflated_first_bgzf_member",
            "confirmed_bam_magic",
        ],
        semantic_note: "Shallow verification confirms BAM-like container and BAM magic only. It does not imply deep BAM validation or EOF completeness.".to_string(),
    })
}
