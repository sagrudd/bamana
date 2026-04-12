use std::{collections::HashSet, path::PathBuf};

use serde::Serialize;

use crate::{
    bam::{
        checksum::{
            ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions, compute_checksums,
        },
        index::IndexKind,
        sort::{QuerynameSubOrder, SortExecutionOptions, SortOrder, sort_bam},
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct SortRequest {
    pub bam: PathBuf,
    pub out: PathBuf,
    pub order: SortOrder,
    pub queryname_suborder: Option<QuerynameSubOrder>,
    pub threads: usize,
    pub memory_limit: Option<u64>,
    pub create_index: bool,
    pub verify_checksum: bool,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct SortPayload {
    pub format: &'static str,
    pub output: SortOutputInfo,
    pub sort: SortResultInfo,
    pub records: SortRecordCounts,
    pub index: SortIndexInfo,
    pub checksum_verification: ChecksumVerificationInfo,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SortOutputInfo {
    pub path: String,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwritten: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SortResultInfo {
    pub requested_order: SortOrder,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_sub_order: Option<QuerynameSubOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produced_order: Option<SortOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produced_sub_order: Option<QuerynameSubOrder>,
}

#[derive(Debug, Serialize)]
pub struct SortRecordCounts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_read: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_written: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SortIndexInfo {
    pub requested: bool,
    pub created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<IndexKind>,
}

#[derive(Debug, Serialize)]
pub struct ChecksumVerificationInfo {
    pub requested: bool,
    pub performed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ChecksumMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<bool>,
}

pub fn run(request: SortRequest) -> CommandResponse<SortPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => return CommandResponse::failure("sort", Some(request.bam.as_path()), error),
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "sort",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "sort",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "sort",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let mut payload = base_payload(&request);

    let sort_result = match sort_bam(&SortExecutionOptions {
        input_path: request.bam.clone(),
        output_path: request.out.clone(),
        force: request.force,
        order: request.order,
        queryname_suborder: request.queryname_suborder,
        threads: request.threads,
        memory_limit: request.memory_limit,
    }) {
        Ok(result) => result,
        Err(error) => {
            return CommandResponse::failure_with_data(
                "sort",
                Some(request.bam.as_path()),
                Some(payload),
                map_transform_error(error, &request.bam),
            );
        }
    };

    payload.output.written = true;
    payload.output.overwritten = Some(sort_result.overwritten);
    payload.sort.produced_order = Some(sort_result.produced_order);
    payload.sort.produced_sub_order = sort_result.produced_sub_order;
    payload.records.records_read = Some(sort_result.records_read);
    payload.records.records_written = Some(sort_result.records_written);
    payload.notes.extend(sort_result.notes);

    update_index_reporting(&request, &mut payload);

    if request.verify_checksum {
        match verify_canonical_checksum(&request.bam, &request.out) {
            Ok((input_digest, output_digest, matched)) => {
                payload.checksum_verification = ChecksumVerificationInfo {
                    requested: true,
                    performed: true,
                    mode: Some(ChecksumMode::CanonicalRecordOrder),
                    input_digest: Some(input_digest.clone()),
                    output_digest: Some(output_digest.clone()),
                    r#match: Some(matched),
                };

                if matched {
                    payload.notes.push(
                        "Canonical checksum verification confirmed record-content preservation under the order-insensitive checksum mode."
                            .to_string(),
                    );
                } else {
                    return CommandResponse::failure_with_data(
                        "sort",
                        Some(request.bam.as_path()),
                        Some(payload),
                        AppError::ChecksumMismatch {
                            path: request.out.clone(),
                            detail: format!(
                                "Input canonical checksum {input_digest} did not match output canonical checksum {output_digest}."
                            ),
                        },
                    );
                }
            }
            Err(error) => {
                return CommandResponse::failure_with_data(
                    "sort",
                    Some(request.bam.as_path()),
                    Some(payload),
                    error,
                );
            }
        }
    }

    CommandResponse::success("sort", Some(request.bam.as_path()), payload)
}

fn base_payload(request: &SortRequest) -> SortPayload {
    SortPayload {
        format: "BAM",
        output: SortOutputInfo {
            path: request.out.to_string_lossy().into_owned(),
            written: false,
            overwritten: None,
        },
        sort: SortResultInfo {
            requested_order: request.order,
            requested_sub_order: request.queryname_suborder,
            produced_order: None,
            produced_sub_order: None,
        },
        records: SortRecordCounts {
            records_read: None,
            records_written: None,
        },
        index: SortIndexInfo {
            requested: request.create_index,
            created: false,
            kind: None,
        },
        checksum_verification: ChecksumVerificationInfo {
            requested: request.verify_checksum,
            performed: false,
            mode: None,
            input_digest: None,
            output_digest: None,
            r#match: None,
        },
        notes: Vec::new(),
    }
}

fn update_index_reporting(request: &SortRequest, payload: &mut SortPayload) {
    if !request.create_index {
        return;
    }

    match request.order {
        SortOrder::Coordinate => {
            payload.index.kind = Some(IndexKind::Bai);
            payload.notes.push(
                "Index creation was requested, but BAI writing is not implemented in this slice."
                    .to_string(),
            );
        }
        SortOrder::Queryname => {
            payload.notes.push(
                "Queryname-sorted BAM output is not suitable for standard coordinate BAM indexing."
                    .to_string(),
            );
        }
    }
}

fn verify_canonical_checksum(
    input: &std::path::Path,
    output: &std::path::Path,
) -> Result<(String, String, bool), AppError> {
    let options = ChecksumOptions {
        mode: ChecksumMode::CanonicalRecordOrder,
        algorithm: ChecksumAlgorithm::Sha256,
        include_header: false,
        excluded_tags: HashSet::new(),
        excluded_tag_strings: Vec::new(),
        filters: ChecksumFilters {
            only_primary: false,
            mapped_only: false,
        },
    };

    let input_digest = extract_digest(
        compute_checksums(input, &options)?,
        ChecksumMode::CanonicalRecordOrder,
    )
    .ok_or_else(|| AppError::ChecksumUncertainty {
        path: input.to_path_buf(),
        detail: "Canonical checksum result was missing from the input BAM checksum response."
            .to_string(),
    })?;
    let output_digest = extract_digest(
        compute_checksums(output, &options)?,
        ChecksumMode::CanonicalRecordOrder,
    )
    .ok_or_else(|| AppError::ChecksumUncertainty {
        path: output.to_path_buf(),
        detail: "Canonical checksum result was missing from the output BAM checksum response."
            .to_string(),
    })?;

    Ok((
        input_digest.clone(),
        output_digest.clone(),
        input_digest == output_digest,
    ))
}

fn extract_digest(
    payload: crate::bam::checksum::ChecksumPayload,
    mode: ChecksumMode,
) -> Option<String> {
    payload
        .results?
        .into_iter()
        .find(|result| result.mode == mode)
        .map(|result| result.digest)
}

fn map_transform_error(error: AppError, input_path: &std::path::Path) -> AppError {
    match error {
        AppError::InvalidHeader { detail, .. }
        | AppError::InvalidRecord { detail, .. }
        | AppError::TruncatedFile { detail, .. } => AppError::ParseUncertainty {
            path: input_path.to_path_buf(),
            detail,
        },
        other => other,
    }
}
