use std::path::PathBuf;

use serde::Serialize;

use crate::{
    bam::{
        scan::BamScanner,
        tags::{AuxTypeCode, TagQuery, record_aux_contains_tag, validate_tag},
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

    let mut scanner = match BamScanner::open(&request.bam) {
        Ok(scanner) => scanner,
        Err(error) => {
            return CommandResponse::failure("check_tag", Some(request.bam.as_path()), error);
        }
    };

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
        match scanner.next_record() {
            Ok(Some(record)) => {
                records_examined += 1;
                match record_aux_contains_tag(&record, query) {
                    Ok(matched) => {
                        if matched {
                            records_with_tag += 1;
                            if !request.count_hits && !request.full_scan {
                                break;
                            }
                        }
                    }
                    Err(detail) => {
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

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::bgzf::test_support::{build_bam_file_with_header_and_records, write_temp_file};

    use super::{CheckTagMode, CheckTagRequest, CheckTagResult, run};

    #[test]
    fn bounded_check_tag_uses_scanner_aux_view() {
        let bam_path = write_temp_file(
            "check-tag-scanner-present",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_record_with_aux(0, 10, "read1", b"NMc\x05ASi*\0\0\0"),
                    build_record_with_aux(0, 20, "read2", b"RGZgroup1\0"),
                ],
            ),
        );

        let response = run(CheckTagRequest {
            bam: bam_path.clone(),
            tag: "NM".to_string(),
            sample_records: 10,
            full_scan: false,
            require_type: Some("c".to_string()),
            count_hits: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("check_tag payload should be present");
        assert!(matches!(payload.mode, CheckTagMode::BoundedScan));
        assert!(matches!(payload.result, CheckTagResult::ObservedPresent));
        assert!(payload.tag_found);
        assert_eq!(payload.records_examined, 1);
        assert_eq!(payload.records_with_tag, 1);
        assert!(!payload.full_file_scanned);
    }

    #[test]
    fn full_check_tag_reports_absence_after_scanner_eof() {
        let bam_path = write_temp_file(
            "check-tag-scanner-absent",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_record_with_aux(0, 10, "read1", b"RGZgroup1\0")],
            ),
        );

        let response = run(CheckTagRequest {
            bam: bam_path.clone(),
            tag: "NM".to_string(),
            sample_records: 1,
            full_scan: true,
            require_type: None,
            count_hits: true,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("check_tag payload should be present");
        assert!(matches!(payload.mode, CheckTagMode::FullScan));
        assert!(matches!(payload.result, CheckTagResult::AbsentInFullScan));
        assert!(!payload.tag_found);
        assert_eq!(payload.records_examined, 1);
        assert_eq!(payload.records_with_tag, 0);
        assert!(payload.full_file_scanned);
    }

    fn build_record_with_aux(ref_id: i32, pos: i32, read_name: &str, aux: &[u8]) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(read_name.as_bytes());
        variable.push(0);
        variable.extend_from_slice(aux);

        let l_read_name = read_name.len() + 1;
        let block_size = 32 + variable.len();
        let bin_mq_nl = l_read_name as u32;
        let flag_nc = 0_u32;

        let mut record = Vec::with_capacity(4 + block_size);
        record.extend_from_slice(&(block_size as i32).to_le_bytes());
        record.extend_from_slice(&ref_id.to_le_bytes());
        record.extend_from_slice(&pos.to_le_bytes());
        record.extend_from_slice(&bin_mq_nl.to_le_bytes());
        record.extend_from_slice(&flag_nc.to_le_bytes());
        record.extend_from_slice(&0_i32.to_le_bytes());
        record.extend_from_slice(&(-1_i32).to_le_bytes());
        record.extend_from_slice(&(-1_i32).to_le_bytes());
        record.extend_from_slice(&0_i32.to_le_bytes());
        record.extend_from_slice(&variable);
        record
    }
}
