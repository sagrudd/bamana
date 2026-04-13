use std::path::PathBuf;

use crate::{
    error::AppError,
    forensics::forensic_inspect::{
        ForensicInspectConfig, ForensicInspectPayload, ForensicInspectionFailure, ForensicScope,
        inspect_path,
    },
    formats::probe::{DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct ForensicInspectRequest {
    pub input: PathBuf,
    pub sample_records: usize,
    pub full_scan: bool,
    pub max_findings: usize,
    pub scopes: ForensicScope,
}

pub fn run(request: ForensicInspectRequest) -> CommandResponse<ForensicInspectPayload> {
    let probe = match probe_path(&request.input) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure(
                "forensic_inspect",
                Some(request.input.as_path()),
                error,
            );
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "forensic_inspect",
            Some(request.input.as_path()),
            AppError::UnknownFormat {
                path: request.input.clone(),
            },
        );
    }

    let config = ForensicInspectConfig {
        record_limit: if request.full_scan {
            u64::MAX
        } else {
            request.sample_records.max(1) as u64
        },
        max_findings: request.max_findings.max(1),
        scopes: request.scopes,
    };

    match inspect_path(&request.input, probe.detected_format, &config) {
        Ok(payload) => {
            CommandResponse::success("forensic_inspect", Some(request.input.as_path()), payload)
        }
        Err(ForensicInspectionFailure { payload, error }) => CommandResponse::failure_with_data(
            "forensic_inspect",
            Some(request.input.as_path()),
            Some(payload),
            error,
        ),
    }
}
