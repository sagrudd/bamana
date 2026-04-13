use std::{cmp::Reverse, collections::HashMap, path::Path};

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    bam::{
        header::parse_bam_header_from_reader,
        reader::BamReader,
        records::{decode_bam_qualities, decode_bam_sequence, read_next_record_layout},
        tags::extract_string_aux_tag,
    },
    error::AppError,
    fastq::{open_fastq_reader, read_next_fastq_record},
    formats::probe::DetectedFormat,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum DuplicationIdentityMode {
    QnameSeq,
    QnameSeqQual,
    QnameSeqQualRg,
}

#[derive(Debug, Clone, Copy)]
pub struct DuplicationScanOptions {
    pub identity_mode: DuplicationIdentityMode,
    pub min_block_size: usize,
    pub max_findings: usize,
    pub record_limit: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdjacentDuplicateBlock {
    pub finding_type: DuplicationFindingType,
    pub first_start: usize,
    pub block_len: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectDuplicationPayload {
    pub format: DetectedFormat,
    pub identity_mode: DuplicationIdentityMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_mode: Option<DuplicationScanMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_examined: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<DuplicationSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<DuplicationFinding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment: Option<DuplicationAssessment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicationScanMode {
    Bounded,
    Full,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicationSummary {
    pub unique_identities: u64,
    pub duplicate_identities: u64,
    pub duplicate_records: u64,
    pub duplicate_instances_beyond_first: u64,
    pub fraction_duplicate_records: f64,
    pub fraction_duplicate_instances_beyond_first: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicationFinding {
    #[serde(rename = "type")]
    pub finding_type: DuplicationFindingType,
    pub confidence: DuplicationConfidence,
    pub evidence_strength: DuplicationEvidenceStrength,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_range_1: Option<DuplicateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_range_2: Option<DuplicateRange>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<DuplicateExample>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicationFindingType {
    ExactRecordDuplicate,
    ContiguousBlockDuplicate,
    NoncontiguousBlockDuplicate,
    WholeFileAppendDuplicate,
    PartialCollectionDuplicate,
    IndeterminateRepetition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DuplicationEvidenceStrength {
    Limited,
    Moderate,
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DuplicationConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateExample {
    pub read_name: String,
    pub sequence_preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_group: Option<String>,
    pub observed_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicationAssessment {
    pub duplication_detected: bool,
    pub likely_operator_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_follow_up: Option<String>,
}

#[derive(Debug)]
pub struct DuplicationInspectionFailure {
    pub payload: InspectDuplicationPayload,
    pub error: AppError,
}

#[derive(Debug)]
struct IdentityStats {
    count: u64,
    example: DuplicateExample,
}

#[derive(Debug)]
struct ScanState {
    identities: Vec<usize>,
    identity_lookup: HashMap<String, usize>,
    identity_stats: Vec<IdentityStats>,
    records_examined: u64,
}

#[derive(Debug, Clone)]
struct InspectableRecord {
    read_name: String,
    sequence: String,
    quality: Option<String>,
    read_group: Option<String>,
}

pub fn inspect_path(
    path: &Path,
    format: DetectedFormat,
    options: DuplicationScanOptions,
) -> Result<InspectDuplicationPayload, DuplicationInspectionFailure> {
    if matches!(format, DetectedFormat::Fastq | DetectedFormat::FastqGz)
        && options.identity_mode == DuplicationIdentityMode::QnameSeqQualRg
    {
        return Err(DuplicationInspectionFailure {
            payload: base_payload(format, options.identity_mode),
            error: AppError::InvalidIdentityMode {
                path: path.to_path_buf(),
                detail: "Identity mode qname_seq_qual_rg requires BAM input because FASTQ records do not carry BAM read-group tags.".to_string(),
            },
        });
    }

    let mut state = ScanState {
        identities: Vec::new(),
        identity_lookup: HashMap::new(),
        identity_stats: Vec::new(),
        records_examined: 0,
    };

    let reached_eof = match format {
        DetectedFormat::Fastq | DetectedFormat::FastqGz => scan_fastq(path, options, &mut state)
            .map_err(|error| {
                failure_with_partial_payload(path, format, options.identity_mode, &state, error)
            })?,
        DetectedFormat::Bam => scan_bam(path, options, &mut state).map_err(|error| {
            failure_with_partial_payload(path, format, options.identity_mode, &state, error)
        })?,
        other => {
            return Err(DuplicationInspectionFailure {
                payload: base_payload(other, options.identity_mode),
                error: AppError::UnsupportedInputForCommand {
                    path: path.to_path_buf(),
                    detail: format!(
                        "inspect_duplication currently supports BAM, FASTQ, and FASTQ.GZ only; detected {other}."
                    ),
                },
            });
        }
    };

    Ok(finalize_payload(format, options, &state, reached_eof))
}

fn scan_fastq(
    path: &Path,
    options: DuplicationScanOptions,
    state: &mut ScanState,
) -> Result<bool, AppError> {
    let mut reader = open_fastq_reader(path)?;

    while state.records_examined < options.record_limit {
        let record = match read_next_fastq_record(&mut reader, path) {
            Ok(Some(record)) => record,
            Ok(None) => return Ok(true),
            Err(AppError::InvalidFastq { detail, .. }) => {
                return Err(AppError::ParseUncertainty {
                    path: path.to_path_buf(),
                    detail,
                });
            }
            Err(error) => return Err(error),
        };

        observe_record(
            state,
            options.identity_mode,
            InspectableRecord {
                read_name: record.read_name,
                sequence: record.sequence,
                quality: Some(record.quality),
                read_group: None,
            },
        );
    }

    Ok(false)
}

fn scan_bam(
    path: &Path,
    options: DuplicationScanOptions,
    state: &mut ScanState,
) -> Result<bool, AppError> {
    let mut reader = BamReader::open(path)?;
    parse_bam_header_from_reader(&mut reader).map_err(|error| AppError::ParseUncertainty {
        path: path.to_path_buf(),
        detail: error.detail().unwrap_or_else(|| error.to_string()),
    })?;

    while state.records_examined < options.record_limit {
        let layout = match read_next_record_layout(&mut reader) {
            Ok(Some(layout)) => layout,
            Ok(None) => return Ok(true),
            Err(
                AppError::InvalidRecord { detail, .. } | AppError::TruncatedFile { detail, .. },
            ) => {
                return Err(AppError::ParseUncertainty {
                    path: path.to_path_buf(),
                    detail,
                });
            }
            Err(error) => return Err(error),
        };

        let sequence =
            decode_bam_sequence(&layout.sequence_bytes, layout.l_seq).map_err(|detail| {
                AppError::ParseUncertainty {
                    path: path.to_path_buf(),
                    detail,
                }
            })?;
        let quality = decode_bam_qualities(&layout.quality_bytes).map_err(|detail| {
            AppError::ParseUncertainty {
                path: path.to_path_buf(),
                detail,
            }
        })?;
        let read_group = if options.identity_mode == DuplicationIdentityMode::QnameSeqQualRg {
            extract_string_aux_tag(&layout.aux_bytes, *b"RG").map_err(|detail| {
                AppError::ParseUncertainty {
                    path: path.to_path_buf(),
                    detail,
                }
            })?
        } else {
            None
        };

        observe_record(
            state,
            options.identity_mode,
            InspectableRecord {
                read_name: layout.read_name,
                sequence,
                quality: Some(quality),
                read_group,
            },
        );
    }

    Ok(false)
}

fn observe_record(
    state: &mut ScanState,
    identity_mode: DuplicationIdentityMode,
    record: InspectableRecord,
) {
    let example = DuplicateExample {
        read_name: record.read_name.clone(),
        sequence_preview: preview_value(&record.sequence),
        quality_preview: record.quality.as_ref().map(|value| preview_value(value)),
        read_group: record.read_group.clone(),
        observed_count: 0,
    };
    let canonical = build_identity_key(
        identity_mode,
        &record.read_name,
        &record.sequence,
        record.quality.as_deref(),
        record.read_group.as_deref(),
    );
    let identity_id = if let Some(existing) = state.identity_lookup.get(&canonical) {
        *existing
    } else {
        let next = state.identity_stats.len();
        state.identity_lookup.insert(canonical, next);
        state
            .identity_stats
            .push(IdentityStats { count: 0, example });
        next
    };

    state.identity_stats[identity_id].count += 1;
    state.identities.push(identity_id);
    state.records_examined += 1;
}

pub(crate) fn build_identity_key(
    identity_mode: DuplicationIdentityMode,
    read_name: &str,
    sequence: &str,
    quality: Option<&str>,
    read_group: Option<&str>,
) -> String {
    match identity_mode {
        DuplicationIdentityMode::QnameSeq => format!("{read_name}\u{1f}{sequence}"),
        DuplicationIdentityMode::QnameSeqQual => format!(
            "{read_name}\u{1f}{sequence}\u{1f}{}",
            quality.unwrap_or("*")
        ),
        DuplicationIdentityMode::QnameSeqQualRg => format!(
            "{read_name}\u{1f}{sequence}\u{1f}{}\u{1f}{}",
            quality.unwrap_or("*"),
            read_group.unwrap_or("")
        ),
    }
}

fn finalize_payload(
    format: DetectedFormat,
    options: DuplicationScanOptions,
    state: &ScanState,
    reached_eof: bool,
) -> InspectDuplicationPayload {
    let summary = build_summary(state);
    let scan_mode = if reached_eof {
        DuplicationScanMode::Full
    } else {
        DuplicationScanMode::Bounded
    };
    let findings = build_findings(state, &summary, options);
    let likely_operator_error = findings.iter().any(|finding| {
        matches!(
            finding.finding_type,
            DuplicationFindingType::WholeFileAppendDuplicate
                | DuplicationFindingType::ContiguousBlockDuplicate
                | DuplicationFindingType::NoncontiguousBlockDuplicate
                | DuplicationFindingType::PartialCollectionDuplicate
        )
    });
    let duplication_detected = summary.duplicate_identities > 0 || !findings.is_empty();
    let recommended_follow_up = if likely_operator_error {
        Some("deduplicate".to_string())
    } else if duplication_detected {
        Some("review_collection_provenance".to_string())
    } else {
        None
    };

    InspectDuplicationPayload {
        format,
        identity_mode: options.identity_mode,
        scan_mode: Some(scan_mode),
        records_examined: Some(state.records_examined),
        summary: Some(summary),
        findings: Some(findings),
        assessment: Some(DuplicationAssessment {
            duplication_detected,
            likely_operator_error,
            recommended_follow_up,
        }),
        notes: Some(build_notes(format, options.identity_mode, reached_eof)),
    }
}

fn build_summary(state: &ScanState) -> DuplicationSummary {
    let unique_identities = state.identity_stats.len() as u64;
    let duplicate_identities = state
        .identity_stats
        .iter()
        .filter(|stats| stats.count > 1)
        .count() as u64;
    let duplicate_records = state
        .identity_stats
        .iter()
        .filter(|stats| stats.count > 1)
        .map(|stats| stats.count)
        .sum::<u64>();
    let duplicate_instances_beyond_first = state
        .identity_stats
        .iter()
        .filter(|stats| stats.count > 1)
        .map(|stats| stats.count - 1)
        .sum::<u64>();
    let records_examined = state.records_examined.max(1);

    DuplicationSummary {
        unique_identities,
        duplicate_identities,
        duplicate_records,
        duplicate_instances_beyond_first,
        fraction_duplicate_records: duplicate_records as f64 / records_examined as f64,
        fraction_duplicate_instances_beyond_first: duplicate_instances_beyond_first as f64
            / records_examined as f64,
    }
}

fn build_findings(
    state: &ScanState,
    summary: &DuplicationSummary,
    options: DuplicationScanOptions,
) -> Vec<DuplicationFinding> {
    let mut findings =
        detect_adjacent_block_duplicates(state, options.min_block_size, options.identity_mode);

    if summary.duplicate_identities > 0 && findings.len() < options.max_findings {
        let finding_type = if findings.is_empty() && summary.fraction_duplicate_records >= 0.25 {
            DuplicationFindingType::PartialCollectionDuplicate
        } else {
            DuplicationFindingType::ExactRecordDuplicate
        };
        let confidence = if summary.fraction_duplicate_records >= 0.25 {
            DuplicationConfidence::High
        } else {
            DuplicationConfidence::Medium
        };
        let examples = duplicate_examples(state);

        findings.push(DuplicationFinding {
            finding_type,
            confidence,
            evidence_strength: DuplicationEvidenceStrength::Moderate,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "{} duplicate record identity signatures were observed under {}.",
                summary.duplicate_identities,
                identity_mode_label(options.identity_mode)
            ),
            examples: (!examples.is_empty()).then_some(examples),
        });
    }

    findings.truncate(options.max_findings);
    findings
}

fn detect_adjacent_block_duplicates(
    state: &ScanState,
    min_block_size: usize,
    identity_mode: DuplicationIdentityMode,
) -> Vec<DuplicationFinding> {
    detect_adjacent_duplicate_blocks(&state.identities, min_block_size)
        .into_iter()
        .map(|block| {
            DuplicationFinding {
                finding_type: block.finding_type,
                confidence: DuplicationConfidence::High,
                evidence_strength: DuplicationEvidenceStrength::Strong,
                record_range_1: Some(DuplicateRange {
                    start: block.first_start as u64 + 1,
                    end: (block.first_start + block.block_len) as u64,
                }),
                record_range_2: Some(DuplicateRange {
                    start: (block.first_start + block.block_len) as u64 + 1,
                    end: (block.first_start + block.block_len * 2) as u64,
                }),
                message: if block.finding_type == DuplicationFindingType::WholeFileAppendDuplicate {
                    format!(
                        "The second half of the examined records appears to duplicate the first half under {} identity.",
                        identity_mode_label(identity_mode)
                    )
                } else {
                    format!(
                        "A contiguous block of {} record identities appears twice in immediate succession under {} identity.",
                        block.block_len,
                        identity_mode_label(identity_mode)
                    )
                },
                examples: None,
            }
        })
        .collect()
}

pub(crate) fn detect_adjacent_duplicate_blocks(
    identities: &[usize],
    min_block_size: usize,
) -> Vec<AdjacentDuplicateBlock> {
    let mut blocks = Vec::new();

    if identities.len() < min_block_size.saturating_mul(2) || min_block_size == 0 {
        return blocks;
    }

    let mut start = 0_usize;
    while start + min_block_size * 2 <= identities.len() {
        if identities[start..start + min_block_size]
            == identities[start + min_block_size..start + min_block_size * 2]
        {
            let mut block_len = min_block_size;
            while start + (block_len + 1) * 2 <= identities.len()
                && identities[start..start + block_len + 1]
                    == identities[start + block_len + 1..start + (block_len + 1) * 2]
            {
                block_len += 1;
            }

            let finding_type = if start == 0 && block_len * 2 == identities.len() {
                DuplicationFindingType::WholeFileAppendDuplicate
            } else {
                DuplicationFindingType::ContiguousBlockDuplicate
            };

            blocks.push(AdjacentDuplicateBlock {
                finding_type,
                first_start: start,
                block_len,
            });
            start += block_len * 2;
        } else {
            start += 1;
        }
    }

    blocks
}

fn duplicate_examples(state: &ScanState) -> Vec<DuplicateExample> {
    let mut examples = state
        .identity_stats
        .iter()
        .filter(|stats| stats.count > 1)
        .map(|stats| {
            let mut example = stats.example.clone();
            example.observed_count = stats.count;
            example
        })
        .collect::<Vec<_>>();

    examples.sort_by_key(|example| {
        (
            Reverse(example.observed_count),
            example.read_name.clone(),
            example.sequence_preview.clone(),
        )
    });
    examples.truncate(3);
    examples
}

fn build_notes(
    format: DetectedFormat,
    identity_mode: DuplicationIdentityMode,
    reached_eof: bool,
) -> Vec<String> {
    let mut notes = vec![
        "inspect_duplication detects collection-duplication and operator-error signatures; it is not Picard/GATK-style PCR duplicate marking.".to_string(),
        "BAM duplicate flags are not used as primary evidence for this command.".to_string(),
        format!(
            "Identity comparisons used {} for {} input.",
            identity_mode_label(identity_mode),
            format
        ),
        "This first slice reports exact duplicate identities and adjacent repeated blocks; non-contiguous repeated-block detection is reserved for a later slice.".to_string(),
    ];

    if !reached_eof {
        notes.push(
            "The scan stopped at the bounded record limit before EOF, so absence of suspicious duplication is not a whole-file proof."
                .to_string(),
        );
    }

    notes
}

fn base_payload(
    format: DetectedFormat,
    identity_mode: DuplicationIdentityMode,
) -> InspectDuplicationPayload {
    InspectDuplicationPayload {
        format,
        identity_mode,
        scan_mode: None,
        records_examined: None,
        summary: None,
        findings: None,
        assessment: None,
        notes: None,
    }
}

fn failure_with_partial_payload(
    path: &Path,
    format: DetectedFormat,
    identity_mode: DuplicationIdentityMode,
    state: &ScanState,
    error: AppError,
) -> DuplicationInspectionFailure {
    let summary = if state.records_examined > 0 {
        Some(build_summary(state))
    } else {
        None
    };

    DuplicationInspectionFailure {
        payload: InspectDuplicationPayload {
            format,
            identity_mode,
            scan_mode: None,
            records_examined: (state.records_examined > 0).then_some(state.records_examined),
            summary,
            findings: None,
            assessment: None,
            notes: Some(vec![
                "Duplication inspection did not complete cleanly enough to support a stable collection-level assessment.".to_string(),
                format!("Input path: {}", path.display()),
            ]),
        },
        error,
    }
}

pub(crate) fn identity_mode_label(mode: DuplicationIdentityMode) -> &'static str {
    match mode {
        DuplicationIdentityMode::QnameSeq => "qname_seq",
        DuplicationIdentityMode::QnameSeqQual => "qname_seq_qual",
        DuplicationIdentityMode::QnameSeqQualRg => "qname_seq_qual_rg",
    }
}

fn preview_value(value: &str) -> String {
    const MAX_PREVIEW: usize = 24;

    if value.len() <= MAX_PREVIEW {
        value.to_string()
    } else {
        format!("{}...", &value[..MAX_PREVIEW])
    }
}

trait AppErrorDetail {
    fn detail(&self) -> Option<String>;
}

impl AppErrorDetail for AppError {
    fn detail(&self) -> Option<String> {
        match self {
            AppError::InvalidHeader { detail, .. }
            | AppError::InvalidRecord { detail, .. }
            | AppError::InvalidFastq { detail, .. }
            | AppError::ParseUncertainty { detail, .. }
            | AppError::TruncatedFile { detail, .. }
            | AppError::InvalidBam { detail, .. } => Some(detail.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{
        bam::{
            header::{ReferenceHeaderFields, ReferenceRecord, serialize_bam_header_payload},
            records::{RecordLayout, encode_bam_qualities, encode_bam_sequence},
            write::{BgzfWriter, serialize_record_layout},
        },
        error::AppError,
        formats::probe::DetectedFormat,
    };

    use super::{
        DuplicateRange, DuplicationFindingType, DuplicationIdentityMode, DuplicationScanMode,
        DuplicationScanOptions, inspect_path,
    };

    #[test]
    fn detects_whole_file_append_in_fastq() {
        let path = std::env::temp_dir().join(format!(
            "bamana-inspect-duplication-fastq-{}.fastq",
            std::process::id()
        ));
        fs::write(
            &path,
            "@r1\nACGT\n+\n!!!!\n@r2\nTGCA\n+\n####\n@r1\nACGT\n+\n!!!!\n@r2\nTGCA\n+\n####\n",
        )
        .expect("fastq fixture should write");

        let payload = inspect_path(
            &path,
            DetectedFormat::Fastq,
            DuplicationScanOptions {
                identity_mode: DuplicationIdentityMode::QnameSeqQual,
                min_block_size: 2,
                max_findings: 10,
                record_limit: u64::MAX,
            },
        )
        .expect("inspection should succeed");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(matches!(payload.scan_mode, Some(DuplicationScanMode::Full)));
        let findings = payload.findings.expect("findings should be present");
        assert!(findings.iter().any(|finding| {
            finding.finding_type == DuplicationFindingType::WholeFileAppendDuplicate
                && finding
                    .record_range_1
                    .as_ref()
                    .map(|range| (range.start, range.end))
                    == Some((1, 2))
                && finding
                    .record_range_2
                    .as_ref()
                    .map(|range| (range.start, range.end))
                    == Some((3, 4))
        }));
    }

    #[test]
    fn detects_adjacent_duplicate_block_in_bam() {
        let path = std::env::temp_dir().join(format!(
            "bamana-inspect-duplication-bam-{}.bam",
            std::process::id()
        ));
        write_test_bam(
            &path,
            vec![
                build_test_record("r1", "ACGT", "!!!!", Some("rg1")),
                build_test_record("r2", "TGCA", "####", Some("rg1")),
                build_test_record("r1", "ACGT", "!!!!", Some("rg1")),
                build_test_record("r2", "TGCA", "####", Some("rg1")),
            ],
        );

        let payload = inspect_path(
            &path,
            DetectedFormat::Bam,
            DuplicationScanOptions {
                identity_mode: DuplicationIdentityMode::QnameSeqQualRg,
                min_block_size: 2,
                max_findings: 10,
                record_limit: u64::MAX,
            },
        )
        .expect("inspection should succeed");
        fs::remove_file(path).expect("fixture should be removable");

        let findings = payload.findings.expect("findings should be present");
        assert!(findings.iter().any(|finding| {
            finding.finding_type == DuplicationFindingType::WholeFileAppendDuplicate
                && finding.record_range_1.as_ref().map(range_tuple) == Some((1, 2))
                && finding.record_range_2.as_ref().map(range_tuple) == Some((3, 4))
        }));
    }

    #[test]
    fn rejects_rg_identity_mode_for_fastq() {
        let path = std::env::temp_dir().join(format!(
            "bamana-inspect-duplication-invalid-{}.fastq",
            std::process::id()
        ));
        fs::write(&path, "@r1\nACGT\n+\n!!!!\n").expect("fastq fixture should write");

        let error = inspect_path(
            &path,
            DetectedFormat::Fastq,
            DuplicationScanOptions {
                identity_mode: DuplicationIdentityMode::QnameSeqQualRg,
                min_block_size: 2,
                max_findings: 10,
                record_limit: 100,
            },
        )
        .expect_err("inspection should fail");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(matches!(error.error, AppError::InvalidIdentityMode { .. }));
    }

    fn range_tuple(range: &DuplicateRange) -> (u64, u64) {
        (range.start, range.end)
    }

    fn build_test_record(
        read_name: &str,
        sequence: &str,
        quality: &str,
        read_group: Option<&str>,
    ) -> RecordLayout {
        let mut aux_bytes = Vec::new();
        if let Some(read_group) = read_group {
            aux_bytes.extend_from_slice(b"RG");
            aux_bytes.push(b'Z');
            aux_bytes.extend_from_slice(read_group.as_bytes());
            aux_bytes.push(0);
        }

        RecordLayout {
            block_size: 0,
            ref_id: -1,
            pos: -1,
            bin: 4680,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags: 0x4,
            mapping_quality: 0,
            n_cigar_op: 0,
            l_seq: sequence.len(),
            read_name: read_name.to_string(),
            cigar_bytes: Vec::new(),
            sequence_bytes: encode_bam_sequence(sequence).expect("sequence should encode"),
            quality_bytes: encode_bam_qualities(quality).expect("quality should encode"),
            aux_bytes,
        }
    }

    fn write_test_bam(path: &PathBuf, records: Vec<RecordLayout>) {
        let header_payload = serialize_bam_header_payload(
            "@HD\tVN:1.6\tSO:unknown\n",
            &[ReferenceRecord {
                name: "chr1".to_string(),
                length: 100,
                index: 0,
                header_fields: ReferenceHeaderFields::default(),
                text_header_length: Some(100),
            }],
        );

        let mut writer = BgzfWriter::create(path).expect("writer should create");
        writer
            .write_all(&header_payload)
            .expect("header should write");
        for record in records {
            writer
                .write_all(&serialize_record_layout(&record))
                .expect("record should write");
        }
        writer.finish().expect("writer should finish");
    }
}
