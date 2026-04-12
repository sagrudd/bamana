use std::{collections::HashSet, path::PathBuf};

use crate::{
    bam::checksum::{
        ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions, ChecksumPayload,
        compute_checksums,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct ChecksumRequest {
    pub bam: PathBuf,
    pub mode: ChecksumMode,
    pub algorithm: ChecksumAlgorithm,
    pub include_header: bool,
    pub exclude_tags: Vec<String>,
    pub only_primary: bool,
    pub mapped_only: bool,
}

pub fn run(request: ChecksumRequest) -> CommandResponse<ChecksumPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("checksum", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "checksum",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "checksum",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "checksum",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let mut excluded_tags = HashSet::new();
    let mut excluded_tag_strings = Vec::new();
    for tag in &request.exclude_tags {
        let Some(validated) = crate::bam::tags::validate_tag(tag) else {
            return CommandResponse::failure(
                "checksum",
                Some(request.bam.as_path()),
                AppError::InvalidTag {
                    path: request.bam.clone(),
                    tag: tag.clone(),
                },
            );
        };
        excluded_tags.insert(validated);
        excluded_tag_strings.push(tag.clone());
    }

    let options = ChecksumOptions {
        mode: request.mode,
        algorithm: request.algorithm,
        include_header: request.include_header,
        excluded_tags,
        excluded_tag_strings,
        filters: ChecksumFilters {
            only_primary: request.only_primary,
            mapped_only: request.mapped_only,
        },
    };

    match compute_checksums(&request.bam, &options) {
        Ok(payload) => CommandResponse::success("checksum", Some(request.bam.as_path()), payload),
        Err(AppError::ChecksumUncertainty { detail, .. }) => {
            let payload = ChecksumPayload {
                format: "BAM",
                algorithm: None,
                results: None,
                semantic_note: None,
            };
            CommandResponse::failure_with_data(
                "checksum",
                Some(request.bam.as_path()),
                Some(payload),
                AppError::ChecksumUncertainty {
                    path: request.bam.clone(),
                    detail,
                },
            )
        }
        Err(error) => CommandResponse::failure("checksum", Some(request.bam.as_path()), error),
    }
}
