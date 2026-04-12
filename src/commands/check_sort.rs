use std::{cmp::Ordering, path::PathBuf};

use serde::Serialize;

use crate::{
    bam::{
        header::{HeaderPayload, parse_bam_header_from_reader},
        reader::BamReader,
        records::{LightAlignmentRecord, read_next_light_record},
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
};

#[derive(Debug)]
pub struct CheckSortRequest {
    pub bam: PathBuf,
    pub sample_records: usize,
    pub strict: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckSortPayload {
    pub format: &'static str,
    pub declared_sort: DeclaredSortInfo,
    pub observed_sort: ObservedSortInfo,
    pub agreement: AgreementInfo,
    pub confidence: ConfidenceLevel,
    pub semantic_note: String,
}

#[derive(Debug, Serialize)]
pub struct DeclaredSortInfo {
    pub so: Option<String>,
    pub ss: Option<String>,
    pub go: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ObservedSortInfo {
    pub order: ObservedOrder,
    pub sub_order: Option<String>,
    pub appears_sorted: Option<bool>,
    pub records_examined: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_violation: Option<FirstViolation>,
    pub evidence_strength: EvidenceStrength,
}

#[derive(Debug, Serialize)]
pub struct AgreementInfo {
    pub header_matches_observation: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FirstViolation {
    pub record_index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceStrength {
    Strong,
    Moderate,
    Limited,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ObservedOrder {
    Coordinate,
    Queryname,
    Unsorted,
    Indeterminate,
}

#[derive(Debug, Default)]
struct ScanState {
    previous: Option<LightAlignmentRecord>,
    coordinate_possible: bool,
    query_lex_possible: bool,
    query_natural_possible: bool,
    records_examined: usize,
    coordinate_violation: Option<FirstViolation>,
    query_lex_violation: Option<FirstViolation>,
    query_natural_violation: Option<FirstViolation>,
}

pub fn run(request: CheckSortRequest) -> Result<CheckSortPayload, AppError> {
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

    let mut reader = BamReader::open(&request.bam)?;
    let header = parse_bam_header_from_reader(&mut reader)?;
    let declared_sort = extract_declared_sort(&header);
    let specialized_mode = declared_specialized_mode(&declared_sort);

    let (scan_state, reached_eof) = scan_records(
        &mut reader,
        request.sample_records.max(1),
        request.strict,
        specialized_mode.is_some(),
    )?;

    let observed_sort = classify_observed_sort(
        &declared_sort,
        specialized_mode.as_deref(),
        &scan_state,
        reached_eof,
        request.sample_records.max(1),
    );
    let agreement = AgreementInfo {
        header_matches_observation: determine_agreement(&declared_sort, &observed_sort),
    };
    let confidence = determine_confidence(&observed_sort, agreement.header_matches_observation);
    let semantic_note = build_semantic_note(
        specialized_mode.as_deref(),
        &observed_sort,
        agreement.header_matches_observation,
        request.strict,
    );

    Ok(CheckSortPayload {
        format: "BAM",
        declared_sort,
        observed_sort,
        agreement,
        confidence,
        semantic_note,
    })
}

fn scan_records(
    reader: &mut BamReader,
    sample_records: usize,
    strict: bool,
    specialized_declared: bool,
) -> Result<(ScanState, bool), AppError> {
    let mut state = ScanState {
        coordinate_possible: true,
        query_lex_possible: true,
        query_natural_possible: true,
        ..ScanState::default()
    };
    let mut reached_eof = false;

    loop {
        if !strict && state.records_examined >= sample_records {
            break;
        }

        match read_next_light_record(reader)? {
            Some(record) => {
                state.records_examined += 1;
                let record_index = state.records_examined;

                if let Some(previous) = &state.previous {
                    if state.coordinate_possible
                        && compare_coordinate(previous, &record) == Ordering::Greater
                    {
                        state.coordinate_possible = false;
                        state.coordinate_violation = Some(FirstViolation {
                            record_index,
                            reason: "Reference/position decreased relative to previous record."
                                .to_string(),
                        });
                    }

                    if state.query_lex_possible && previous.read_name > record.read_name {
                        state.query_lex_possible = false;
                        state.query_lex_violation = Some(FirstViolation {
                            record_index,
                            reason: "Read name decreased in lexicographical order.".to_string(),
                        });
                    }

                    if state.query_natural_possible
                        && compare_natural(&previous.read_name, &record.read_name)
                            == Ordering::Greater
                    {
                        state.query_natural_possible = false;
                        state.query_natural_violation = Some(FirstViolation {
                            record_index,
                            reason: "Read name decreased in natural-order comparison.".to_string(),
                        });
                    }
                }

                state.previous = Some(record);

                if !state.coordinate_possible
                    && !state.query_lex_possible
                    && !state.query_natural_possible
                {
                    break;
                }

                if !strict && specialized_declared && state.records_examined >= sample_records {
                    break;
                }
            }
            None => {
                reached_eof = true;
                break;
            }
        }
    }

    Ok((state, reached_eof))
}

fn classify_observed_sort(
    declared_sort: &DeclaredSortInfo,
    specialized_mode: Option<&str>,
    scan_state: &ScanState,
    reached_eof: bool,
    sample_records: usize,
) -> ObservedSortInfo {
    let specialized_template = specialized_mode.is_some_and(|mode| mode == "template-coordinate");
    let specialized_minhash = specialized_mode.is_some_and(|mode| mode.contains("minhash"));

    if scan_state.records_examined == 0 {
        return ObservedSortInfo {
            order: ObservedOrder::Indeterminate,
            sub_order: specialized_mode.map(ToOwned::to_owned),
            appears_sorted: None,
            records_examined: 0,
            first_violation: None,
            evidence_strength: EvidenceStrength::Limited,
        };
    }

    if !scan_state.coordinate_possible
        && !scan_state.query_lex_possible
        && !scan_state.query_natural_possible
    {
        return ObservedSortInfo {
            order: ObservedOrder::Unsorted,
            sub_order: specialized_mode.map(ToOwned::to_owned),
            appears_sorted: Some(false),
            records_examined: scan_state.records_examined,
            first_violation: earliest_violation(scan_state),
            evidence_strength: EvidenceStrength::Strong,
        };
    }

    if specialized_template {
        return ObservedSortInfo {
            order: ObservedOrder::Indeterminate,
            sub_order: specialized_mode.map(ToOwned::to_owned),
            appears_sorted: None,
            records_examined: scan_state.records_examined,
            first_violation: None,
            evidence_strength: if reached_eof {
                EvidenceStrength::Moderate
            } else {
                EvidenceStrength::Limited
            },
        };
    }

    if specialized_minhash && !scan_state.coordinate_possible {
        return ObservedSortInfo {
            order: ObservedOrder::Unsorted,
            sub_order: specialized_mode.map(ToOwned::to_owned),
            appears_sorted: Some(false),
            records_examined: scan_state.records_examined,
            first_violation: scan_state.coordinate_violation.clone(),
            evidence_strength: EvidenceStrength::Strong,
        };
    }

    let query_possible = scan_state.query_lex_possible || scan_state.query_natural_possible;
    let declared_order = declared_sort.so.as_deref();

    if scan_state.coordinate_possible && !query_possible {
        return ObservedSortInfo {
            order: ObservedOrder::Coordinate,
            sub_order: specialized_mode
                .map(ToOwned::to_owned)
                .filter(|_| specialized_minhash),
            appears_sorted: Some(true),
            records_examined: scan_state.records_examined,
            first_violation: None,
            evidence_strength: evidence_strength_for_positive(
                reached_eof,
                scan_state.records_examined,
                sample_records,
                specialized_mode.is_some(),
            ),
        };
    }

    if !scan_state.coordinate_possible && query_possible {
        return ObservedSortInfo {
            order: ObservedOrder::Queryname,
            sub_order: observed_query_suborder(scan_state),
            appears_sorted: Some(true),
            records_examined: scan_state.records_examined,
            first_violation: None,
            evidence_strength: evidence_strength_for_positive(
                reached_eof,
                scan_state.records_examined,
                sample_records,
                false,
            ),
        };
    }

    match declared_order {
        Some("coordinate") if scan_state.coordinate_possible => ObservedSortInfo {
            order: ObservedOrder::Coordinate,
            sub_order: specialized_mode
                .map(ToOwned::to_owned)
                .filter(|_| specialized_minhash),
            appears_sorted: Some(true),
            records_examined: scan_state.records_examined,
            first_violation: None,
            evidence_strength: evidence_strength_for_positive(
                reached_eof,
                scan_state.records_examined,
                sample_records,
                specialized_mode.is_some(),
            ),
        },
        Some("queryname") if query_possible => ObservedSortInfo {
            order: ObservedOrder::Queryname,
            sub_order: observed_query_suborder(scan_state),
            appears_sorted: Some(true),
            records_examined: scan_state.records_examined,
            first_violation: None,
            evidence_strength: evidence_strength_for_positive(
                reached_eof,
                scan_state.records_examined,
                sample_records,
                false,
            ),
        },
        _ => ObservedSortInfo {
            order: ObservedOrder::Indeterminate,
            sub_order: specialized_mode.map(ToOwned::to_owned),
            appears_sorted: None,
            records_examined: scan_state.records_examined,
            first_violation: None,
            evidence_strength: if reached_eof && scan_state.records_examined >= sample_records / 2 {
                EvidenceStrength::Moderate
            } else {
                EvidenceStrength::Limited
            },
        },
    }
}

fn evidence_strength_for_positive(
    reached_eof: bool,
    records_examined: usize,
    sample_records: usize,
    specialized_mode: bool,
) -> EvidenceStrength {
    if specialized_mode {
        if reached_eof {
            EvidenceStrength::Moderate
        } else {
            EvidenceStrength::Limited
        }
    } else if reached_eof || records_examined >= sample_records {
        EvidenceStrength::Strong
    } else if records_examined >= sample_records / 2 || records_examined >= 250 {
        EvidenceStrength::Moderate
    } else {
        EvidenceStrength::Limited
    }
}

fn observed_query_suborder(scan_state: &ScanState) -> Option<String> {
    match (
        scan_state.query_natural_possible,
        scan_state.query_lex_possible,
    ) {
        (true, false) => Some("queryname:natural".to_string()),
        (false, true) => Some("queryname:lexicographical".to_string()),
        _ => None,
    }
}

fn earliest_violation(scan_state: &ScanState) -> Option<FirstViolation> {
    [
        scan_state.coordinate_violation.as_ref(),
        scan_state.query_lex_violation.as_ref(),
        scan_state.query_natural_violation.as_ref(),
    ]
    .into_iter()
    .flatten()
    .min_by_key(|violation| violation.record_index)
    .cloned()
}

fn determine_agreement(
    declared_sort: &DeclaredSortInfo,
    observed_sort: &ObservedSortInfo,
) -> Option<bool> {
    let declared_so = declared_sort.so.as_deref()?;

    match declared_so {
        "coordinate" => match observed_sort.order {
            ObservedOrder::Coordinate if observed_sort.appears_sorted == Some(true) => Some(true),
            ObservedOrder::Coordinate => None,
            ObservedOrder::Queryname | ObservedOrder::Unsorted => Some(false),
            ObservedOrder::Indeterminate => None,
        },
        "queryname" => match observed_sort.order {
            ObservedOrder::Queryname if observed_sort.appears_sorted == Some(true) => Some(true),
            ObservedOrder::Queryname => None,
            ObservedOrder::Coordinate | ObservedOrder::Unsorted => Some(false),
            ObservedOrder::Indeterminate => None,
        },
        "unsorted" => match observed_sort.order {
            ObservedOrder::Unsorted => Some(true),
            _ => None,
        },
        _ => None,
    }
}

fn determine_confidence(
    observed_sort: &ObservedSortInfo,
    agreement: Option<bool>,
) -> ConfidenceLevel {
    match (
        observed_sort.evidence_strength,
        agreement,
        observed_sort.order,
    ) {
        (EvidenceStrength::Strong, Some(true), _) | (EvidenceStrength::Strong, Some(false), _) => {
            ConfidenceLevel::High
        }
        (EvidenceStrength::Strong, _, ObservedOrder::Unsorted) => ConfidenceLevel::High,
        (EvidenceStrength::Strong, _, ObservedOrder::Coordinate | ObservedOrder::Queryname) => {
            ConfidenceLevel::High
        }
        (EvidenceStrength::Moderate, _, _) => ConfidenceLevel::Medium,
        (EvidenceStrength::Limited, _, ObservedOrder::Indeterminate) => ConfidenceLevel::Low,
        _ => ConfidenceLevel::Medium,
    }
}

fn build_semantic_note(
    specialized_mode: Option<&str>,
    _observed_sort: &ObservedSortInfo,
    agreement: Option<bool>,
    strict: bool,
) -> String {
    let base = if strict {
        "Assessment is based on BAM header metadata plus sequential inspection of alignment records until EOF or a stronger conclusion was reached; it is not a full validation of every record in the file."
    } else {
        "Assessment is based on BAM header metadata plus a bounded scan of alignment records; it is not a full validation of every record in the file."
    };

    if let Some(mode) = specialized_mode {
        if mode == "template-coordinate" || mode.contains("minhash") {
            return format!(
                "Header indicates a specialized sort mode ({mode}) that is preserved in output, but full observed confirmation is not implemented in this slice. {base}"
            );
        }
    }

    if agreement == Some(false) {
        return format!("Observed ordering contradicts the BAM header sort declaration. {base}");
    }

    base.to_string()
}

fn extract_declared_sort(header: &HeaderPayload) -> DeclaredSortInfo {
    DeclaredSortInfo {
        so: header.header.hd.sort_order.clone(),
        ss: header.header.hd.sub_sort_order.clone(),
        go: header.header.hd.group_order.clone(),
    }
}

fn declared_specialized_mode(declared_sort: &DeclaredSortInfo) -> Option<String> {
    let ss = declared_sort.ss.as_deref()?;
    if ss.contains("minhash") || ss == "template-coordinate" {
        Some(ss.to_string())
    } else {
        None
    }
}

fn compare_coordinate(previous: &LightAlignmentRecord, current: &LightAlignmentRecord) -> Ordering {
    coordinate_key(previous).cmp(&coordinate_key(current))
}

fn coordinate_key(record: &LightAlignmentRecord) -> (u8, i32, i32, u8, u8, u8, u8, u8, u16) {
    if record.ref_id < 0 {
        (
            1,
            i32::MAX,
            i32::MAX,
            u8::from(record.is_reverse),
            u8::from(record.is_secondary),
            u8::from(record.is_supplementary),
            u8::from(record.is_read1),
            u8::from(record.is_read2),
            record.flags,
        )
    } else {
        (
            0,
            record.ref_id,
            record.pos.max(0),
            u8::from(record.is_reverse),
            u8::from(record.is_secondary),
            u8::from(record.is_supplementary),
            u8::from(record.is_read1),
            u8::from(record.is_read2),
            record.flags,
        )
    }
}

fn compare_natural(left: &str, right: &str) -> Ordering {
    use std::cmp::Ordering::{Equal, Greater, Less};

    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    let mut i = 0;
    let mut j = 0;

    while i < left_bytes.len() && j < right_bytes.len() {
        let left_byte = left_bytes[i];
        let right_byte = right_bytes[j];

        if left_byte.is_ascii_digit() && right_byte.is_ascii_digit() {
            let left_start = i;
            let right_start = j;

            while i < left_bytes.len() && left_bytes[i].is_ascii_digit() {
                i += 1;
            }
            while j < right_bytes.len() && right_bytes[j].is_ascii_digit() {
                j += 1;
            }

            let left_digits = &left_bytes[left_start..i];
            let right_digits = &right_bytes[right_start..j];
            let left_trimmed = trim_leading_zeroes(left_digits);
            let right_trimmed = trim_leading_zeroes(right_digits);

            match left_trimmed.len().cmp(&right_trimmed.len()) {
                Equal => match left_trimmed.cmp(right_trimmed) {
                    Equal => match left_digits.len().cmp(&right_digits.len()) {
                        Equal => continue,
                        other => return other,
                    },
                    other => return other,
                },
                other => return other,
            }
        } else {
            match left_byte.cmp(&right_byte) {
                Equal => {
                    i += 1;
                    j += 1;
                }
                Less => return Less,
                Greater => return Greater,
            }
        }
    }

    left_bytes.len().cmp(&right_bytes.len())
}

fn trim_leading_zeroes(digits: &[u8]) -> &[u8] {
    let trimmed = digits
        .iter()
        .position(|digit| *digit != b'0')
        .unwrap_or(digits.len().saturating_sub(1));
    &digits[trimmed..]
}

#[cfg(test)]
mod tests {
    use super::{CheckSortRequest, ObservedOrder, run};
    use crate::formats::bgzf::test_support::{
        build_bam_file_with_header_and_records, build_light_record, write_temp_file,
    };

    #[test]
    fn recognizes_coordinate_sort_from_header_and_records() {
        let header_text = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n";
        let bytes = build_bam_file_with_header_and_records(
            header_text,
            &[("chr1", 1000)],
            &[
                build_light_record(0, 10, "read1", 0),
                build_light_record(0, 20, "read2", 0),
                build_light_record(0, 30, "read3", 16),
            ],
        );
        let path = write_temp_file("check-sort-coordinate", "bam", &bytes);

        let result = run(CheckSortRequest {
            bam: path.clone(),
            sample_records: 10,
            strict: false,
        })
        .expect("check_sort should succeed");

        std::fs::remove_file(path).expect("fixture should be removed");
        assert!(matches!(
            result.observed_sort.order,
            ObservedOrder::Coordinate
        ));
        assert_eq!(result.agreement.header_matches_observation, Some(true));
    }

    #[test]
    fn detects_unsorted_coordinate_violation() {
        let header_text = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n";
        let bytes = build_bam_file_with_header_and_records(
            header_text,
            &[("chr1", 1000)],
            &[
                build_light_record(0, 50, "read1", 0),
                build_light_record(0, 40, "read2", 0),
            ],
        );
        let path = write_temp_file("check-sort-unsorted", "bam", &bytes);

        let result = run(CheckSortRequest {
            bam: path.clone(),
            sample_records: 10,
            strict: false,
        })
        .expect("check_sort should succeed");

        std::fs::remove_file(path).expect("fixture should be removed");
        assert!(matches!(
            result.observed_sort.order,
            ObservedOrder::Unsorted
        ));
        assert_eq!(result.agreement.header_matches_observation, Some(false));
    }

    #[test]
    fn recognizes_queryname_sort() {
        let header_text =
            "@HD\tVN:1.6\tSO:queryname\tSS:queryname:natural\n@SQ\tSN:chr1\tLN:1000\n";
        let bytes = build_bam_file_with_header_and_records(
            header_text,
            &[("chr1", 1000)],
            &[
                build_light_record(0, 10, "read1", 0),
                build_light_record(0, 10, "read2", 0),
                build_light_record(0, 10, "read10", 0),
            ],
        );
        let path = write_temp_file("check-sort-query", "bam", &bytes);

        let result = run(CheckSortRequest {
            bam: path.clone(),
            sample_records: 10,
            strict: false,
        })
        .expect("check_sort should succeed");

        std::fs::remove_file(path).expect("fixture should be removed");
        assert!(matches!(
            result.observed_sort.order,
            ObservedOrder::Queryname
        ));
        assert_eq!(
            result.observed_sort.sub_order.as_deref(),
            Some("queryname:natural")
        );
    }
}
