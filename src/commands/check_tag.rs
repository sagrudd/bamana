use std::path::PathBuf;

use serde::Serialize;

use crate::{
    bam::{
        header::parse_bam_header_from_reader,
        reader::BamReader,
        tags::{AuxTypeCode, TagQuery, read_next_record_for_tag, validate_tag},
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct CheckTagRequest {
    pub bam: PathBuf,
    pub tag: String,
    pub sample_records: usize,
    pub full_scan: bool,
    pub require_type: Option<String>,
    pub count_hits: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckTagPayload {
    pub format: &'static str,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_type: Option<String>,
    pub mode: CheckTagMode,
    pub result: CheckTagResult,
    pub tag_found: bool,
    pub records_examined: u64,
    pub records_with_tag: u64,
    pub full_file_scanned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckTagMode {
    BoundedScan,
    FullScan,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckTagResult {
    ObservedPresent,
    NotFoundInExaminedRecords,
    AbsentInFullScan,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

pub fn run(request: CheckTagRequest) -> CommandResponse<CheckTagPayload> {
    let mode = if request.full_scan {
        CheckTagMode::FullScan
    } else {
        CheckTagMode::BoundedScan
    };

    let tag = match validate_tag(&request.tag) {
        Some(tag) => tag,
        None => {
            return CommandResponse::failure(
                "check_tag",
                Some(request.bam.as_path()),
                AppError::InvalidTag {
                    path: request.bam.clone(),
                    tag: request.tag.clone(),
                },
            );
        }
    };

    let required_type = match request.require_type.as_deref() {
        Some(type_code) => match AuxTypeCode::parse(type_code) {
            Some(parsed) => Some(parsed),
            None => {
                return CommandResponse::failure(
                    "check_tag",
                    Some(request.bam.as_path()),
                    AppError::InvalidTagType {
                        path: request.bam.clone(),
                        tag_type: type_code.to_string(),
                    },
                );
            }
        },
        None => None,
    };

    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("check_tag", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "check_tag",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "check_tag",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "check_tag",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let mut reader = match BamReader::open(&request.bam) {
        Ok(reader) => reader,
        Err(error) => {
            return CommandResponse::failure("check_tag", Some(request.bam.as_path()), error);
        }
    };
    if let Err(error) = parse_bam_header_from_reader(&mut reader) {
        return CommandResponse::failure("check_tag", Some(request.bam.as_path()), error);
    }

    let query = TagQuery { tag, required_type };
    let record_limit = if request.full_scan {
        u64::MAX
    } else {
        request.sample_records.max(1) as u64
    };

    let mut records_examined = 0_u64;
    let mut records_with_tag = 0_u64;
    let mut full_file_scanned = false;

    while records_examined < record_limit {
        match read_next_record_for_tag(&mut reader, query) {
            Ok(Some(result)) => {
                records_examined += 1;
                if result.matched {
                    records_with_tag += 1;
                    if !request.count_hits && !request.full_scan {
                        break;
                    }
                }
            }
            Ok(None) => {
                full_file_scanned = true;
                break;
            }
            Err(
                AppError::InvalidRecord { detail, .. } | AppError::TruncatedFile { detail, .. },
            ) => {
                let payload = CheckTagPayload {
                    format: "BAM",
                    tag: request.tag.clone(),
                    required_type: request.require_type.clone(),
                    mode,
                    result: CheckTagResult::Indeterminate,
                    tag_found: records_with_tag > 0,
                    records_examined,
                    records_with_tag,
                    full_file_scanned: false,
                    confidence: Some(ConfidenceLevel::Low),
                    semantic_note: Some(
                        "Auxiliary-tag inspection became indeterminate before a stable conclusion was reached."
                            .to_string(),
                    ),
                };
                return CommandResponse::failure_with_data(
                    "check_tag",
                    Some(request.bam.as_path()),
                    Some(payload),
                    AppError::TagParseUncertainty {
                        path: request.bam.clone(),
                        detail,
                    },
                );
            }
            Err(error) => {
                return CommandResponse::failure("check_tag", Some(request.bam.as_path()), error);
            }
        }
    }

    let result = if records_with_tag > 0 {
        CheckTagResult::ObservedPresent
    } else if full_file_scanned {
        CheckTagResult::AbsentInFullScan
    } else {
        CheckTagResult::NotFoundInExaminedRecords
    };

    let confidence = match result {
        CheckTagResult::ObservedPresent => ConfidenceLevel::High,
        CheckTagResult::AbsentInFullScan => ConfidenceLevel::High,
        CheckTagResult::NotFoundInExaminedRecords => ConfidenceLevel::Medium,
        CheckTagResult::Indeterminate => ConfidenceLevel::Low,
    };

    let semantic_note = build_semantic_note(
        result,
        request.require_type.as_deref(),
        full_file_scanned,
        request.count_hits,
    );

    CommandResponse::success(
        "check_tag",
        Some(request.bam.as_path()),
        CheckTagPayload {
            format: "BAM",
            tag: request.tag,
            required_type: request.require_type,
            mode,
            result,
            tag_found: records_with_tag > 0,
            records_examined,
            records_with_tag,
            full_file_scanned,
            confidence: Some(confidence),
            semantic_note: Some(semantic_note),
        },
    )
}

fn build_semantic_note(
    result: CheckTagResult,
    required_type: Option<&str>,
    full_file_scanned: bool,
    count_hits: bool,
) -> String {
    match result {
        CheckTagResult::ObservedPresent => {
            if required_type.is_some() {
                if full_file_scanned && count_hits {
                    "The requested tag was observed with the required auxiliary type during a complete scan of the alignment records."
                        .to_string()
                } else {
                    "The requested tag was observed with the required auxiliary type in the examined records."
                        .to_string()
                }
            } else if full_file_scanned && count_hits {
                "The requested tag was observed during a complete scan of the alignment records."
                    .to_string()
            } else {
                "The requested tag was observed in the examined records. This establishes presence but does not quantify prevalence across the full file."
                    .to_string()
            }
        }
        CheckTagResult::NotFoundInExaminedRecords => {
            "The requested tag was not found in the examined records. This does not prove absence from the full file."
                .to_string()
        }
        CheckTagResult::AbsentInFullScan => {
            "The requested tag was not found during a complete successful scan of the alignment records."
                .to_string()
        }
        CheckTagResult::Indeterminate => {
            "Auxiliary-tag inspection became indeterminate before a stable conclusion was reached."
                .to_string()
        }
    }
}
