use std::path::PathBuf;

use crate::{
    error::AppError,
    forensics::duplication::{
        DuplicationInspectionFailure, DuplicationScanOptions, InspectDuplicationPayload,
        inspect_path,
    },
    formats::probe::{DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct InspectDuplicationRequest {
    pub input: PathBuf,
    pub options: DuplicationScanOptions,
}

pub fn run(request: InspectDuplicationRequest) -> CommandResponse<InspectDuplicationPayload> {
    let probe = match probe_path(&request.input) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure(
                "inspect_duplication",
                Some(request.input.as_path()),
                error,
            );
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "inspect_duplication",
            Some(request.input.as_path()),
            AppError::UnknownFormat {
                path: request.input.clone(),
            },
        );
    }

    match inspect_path(&request.input, probe.detected_format, request.options) {
        Ok(payload) => CommandResponse::success(
            "inspect_duplication",
            Some(request.input.as_path()),
            payload,
        ),
        Err(DuplicationInspectionFailure { payload, error }) => CommandResponse::failure_with_data(
            "inspect_duplication",
            Some(request.input.as_path()),
            Some(payload),
            error,
        ),
    }
}
