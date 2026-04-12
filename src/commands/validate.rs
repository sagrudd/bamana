use std::path::PathBuf;

use crate::{
    bam::validate::{
        ValidatePayload, ValidationMode, ValidationOptions, ValidationSummary, validate_bam,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct ValidateRequest {
    pub bam: PathBuf,
    pub max_errors: usize,
    pub max_warnings: usize,
    pub header_only: bool,
    pub records: Option<u64>,
    pub fail_fast: bool,
    pub include_warnings: bool,
}

pub fn run(request: ValidateRequest) -> CommandResponse<ValidatePayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("validate", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "validate",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "validate",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "validate",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let mode = if request.header_only {
        ValidationMode::HeaderOnly
    } else if request.records.is_some() {
        ValidationMode::BoundedRecords
    } else {
        ValidationMode::Full
    };

    let payload = match validate_bam(
        &request.bam,
        ValidationOptions {
            max_errors: request.max_errors,
            max_warnings: request.max_warnings,
            header_only: request.header_only,
            record_limit: request.records,
            fail_fast: request.fail_fast,
            include_warnings: request.include_warnings,
        },
    ) {
        Ok(payload) => payload,
        Err(error) => {
            let fallback = ValidatePayload {
                format: "BAM",
                mode,
                valid: false,
                summary: ValidationSummary {
                    header_valid: false,
                    records_examined: 0,
                    full_file_examined: false,
                    errors: 1,
                    warnings: 0,
                    infos: 0,
                },
                findings: Vec::new(),
                semantic_note: "Validation could not complete because the BAM could not be opened or parsed to the requested scope.".to_string(),
            };
            return CommandResponse::failure_with_data(
                "validate",
                Some(request.bam.as_path()),
                Some(fallback),
                error,
            );
        }
    };

    if payload.valid {
        CommandResponse::success("validate", Some(request.bam.as_path()), payload)
    } else {
        CommandResponse::failure_with_data(
            "validate",
            Some(request.bam.as_path()),
            Some(payload),
            AppError::ValidationFailed {
                path: request.bam.clone(),
                detail: "Structural validation encountered one or more error-level findings."
                    .to_string(),
            },
        )
    }
}
