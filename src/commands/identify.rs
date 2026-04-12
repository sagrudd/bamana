use std::path::PathBuf;

use serde::Serialize;

use crate::{
    error::AppError,
    formats::probe::{Confidence, ContainerKind, DetectedFormat, probe_path},
};

#[derive(Debug)]
pub struct IdentifyRequest {
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct IdentifyResponse {
    pub detected_format: DetectedFormat,
    pub container: ContainerKind,
    pub confidence: Confidence,
}

pub fn run(request: IdentifyRequest) -> Result<IdentifyResponse, AppError> {
    let probe = probe_path(&request.path)?;
    if probe.detected_format == DetectedFormat::Unknown {
        return Err(AppError::UnknownFormat { path: request.path });
    }

    Ok(IdentifyResponse {
        detected_format: probe.detected_format,
        container: probe.container,
        confidence: probe.confidence,
    })
}
