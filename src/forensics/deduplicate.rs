use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    bam::{
        checksum::{
            ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions, compute_checksums,
            extract_digest,
        },
        header::{HeaderPayload, parse_bam_header_from_reader, serialize_bam_header_payload},
        index::{IndexKind, IndexResolution, ResolvedIndex, resolve_index_for_bam},
        reader::BamReader,
        records::{
            RecordLayout, decode_bam_qualities, decode_bam_sequence, read_next_record_layout,
        },
        tags::extract_string_aux_tag,
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
    forensics::duplication::{
        DuplicateRange, DuplicationFindingType, DuplicationIdentityMode,
        build_identity_key as build_duplication_identity_key, detect_adjacent_duplicate_blocks,
        identity_mode_label,
    },
    formats::probe::DetectedFormat,
    ingest::fastq::{FastqRecord, open_fastq_reader, read_next_fastq_record, write_fastq_records},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[value(rename_all = "kebab-case")]
pub enum DeduplicateMode {
    ContiguousBlock,
    WholeFileAppend,
    GlobalExact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum DeduplicateKeepPolicy {
    First,
    Last,
}

#[derive(Debug, Clone)]
pub struct DeduplicateConfig {
    pub input: PathBuf,
    pub out: PathBuf,
    pub mode: DeduplicateMode,
    pub identity_mode: DuplicationIdentityMode,
    pub keep_policy: DeduplicateKeepPolicy,
    pub dry_run: bool,
    pub force: bool,
    pub min_block_size: usize,
    pub verify_checksum: bool,
    pub emit_removed_report: Option<PathBuf>,
    pub sample_records: usize,
    pub full_scan: bool,
    pub reindex: bool,
    pub json_pretty: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeduplicatePayload {
    pub format: DetectedFormat,
    pub mode: DeduplicateMode,
    pub identity_mode: DuplicationIdentityMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_policy: Option<DeduplicateKeepPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<DeduplicateExecutionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<DeduplicateSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranges: Option<Vec<DeduplicateRange>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<DeduplicateOutputInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<DeduplicateIndexInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_verification: Option<DeduplicateChecksumVerificationInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeduplicateExecutionInfo {
    pub dry_run: bool,
    pub modified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeduplicateSummary {
    pub records_examined: u64,
    pub duplicate_ranges_detected: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_marked_for_removal: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_retained_if_applied: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_removed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_retained: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeduplicateRange {
    #[serde(rename = "type")]
    pub range_type: DeduplicationRangeType,
    pub keep_range: DuplicateRange,
    pub remove_range: DuplicateRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeduplicationRangeType {
    ContiguousBlockDuplicate,
    WholeFileAppendDuplicate,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeduplicateOutputInfo {
    pub path: String,
    pub written: bool,
    pub overwritten: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeduplicateIndexInfo {
    pub present_before: bool,
    pub valid_after: bool,
    pub reindex_requested: bool,
    pub reindexed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<IndexKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeduplicateChecksumVerificationInfo {
    pub requested: bool,
    pub performed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ChecksumMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,
    #[serde(rename = "match")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug)]
pub struct DeduplicateFailure {
    pub payload: DeduplicatePayload,
    pub error: AppError,
}

#[derive(Debug, Serialize)]
struct RemovedReport {
    pub input_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    pub format: DetectedFormat,
    pub mode: DeduplicateMode,
    pub identity_mode: DuplicationIdentityMode,
    pub keep_policy: DeduplicateKeepPolicy,
    pub dry_run: bool,
    pub summary: DeduplicateSummary,
    pub ranges: Vec<DeduplicateRange>,
}

#[derive(Debug)]
enum LoadedInput {
    Fastq {
        records: Vec<FastqRecord>,
    },
    Bam {
        header: HeaderPayload,
        records: Vec<RecordLayout>,
        index_before: Option<ResolvedIndex>,
    },
}

#[derive(Debug)]
struct LoadedScan {
    format: DetectedFormat,
    identities: Vec<usize>,
    records_examined: u64,
    reached_eof: bool,
    input: LoadedInput,
}

pub fn execute(config: &DeduplicateConfig) -> Result<DeduplicatePayload, DeduplicateFailure> {
    let detected_format = crate::formats::probe::probe_path(&config.input)
        .map(|probe| probe.detected_format)
        .unwrap_or(DetectedFormat::Unknown);

    if config.mode == DeduplicateMode::GlobalExact {
        return Err(DeduplicateFailure {
            payload: base_payload(
                detected_format,
                config.mode,
                config.identity_mode,
                config.keep_policy,
            ),
            error: AppError::Unimplemented {
                path: config.input.clone(),
                detail:
                    "Deduplicate mode global-exact is reserved for a later, more aggressive slice."
                        .to_string(),
            },
        });
    }

    if !config.dry_run && config.input == config.out {
        return Err(DeduplicateFailure {
            payload: base_payload(
                detected_format,
                config.mode,
                config.identity_mode,
                config.keep_policy,
            ),
            error: AppError::InvalidDeduplicateMode {
                path: config.input.clone(),
                detail: "Output path must differ from the input path for deduplicate.".to_string(),
            },
        });
    }

    let scan = load_input(config).map_err(|error| DeduplicateFailure {
        payload: base_payload(
            detected_format_from_error(config, &error),
            config.mode,
            config.identity_mode,
            config.keep_policy,
        ),
        error,
    })?;

    let ranges = build_ranges(config, &scan.identities);
    let records_removed = ranges
        .iter()
        .map(|range| range.remove_range.end - range.remove_range.start + 1)
        .sum::<u64>();
    let records_retained = scan.records_examined.saturating_sub(records_removed);
    let modified = !ranges.is_empty();

    let summary = if config.dry_run {
        DeduplicateSummary {
            records_examined: scan.records_examined,
            duplicate_ranges_detected: ranges.len(),
            records_marked_for_removal: Some(records_removed),
            records_retained_if_applied: Some(records_retained),
            records_removed: None,
            records_retained: None,
        }
    } else {
        DeduplicateSummary {
            records_examined: scan.records_examined,
            duplicate_ranges_detected: ranges.len(),
            records_marked_for_removal: None,
            records_retained_if_applied: None,
            records_removed: Some(records_removed),
            records_retained: Some(records_retained),
        }
    };

    let mut notes = build_notes(config, &scan, modified);
    let mut output = None;
    let mut index = None;
    let mut checksum_verification = Some(DeduplicateChecksumVerificationInfo {
        requested: config.verify_checksum,
        performed: false,
        mode: None,
        input_digest: None,
        output_digest: None,
        matched: None,
        notes: Vec::new(),
    });

    if !config.dry_run {
        let overwritten = config.out.exists();
        write_output(config, &scan, &ranges).map_err(|error| DeduplicateFailure {
            payload: DeduplicatePayload {
                format: scan.format,
                mode: config.mode,
                identity_mode: config.identity_mode,
                keep_policy: Some(config.keep_policy),
                execution: Some(DeduplicateExecutionInfo {
                    dry_run: false,
                    modified,
                }),
                summary: Some(summary.clone()),
                ranges: Some(ranges.clone()),
                output: None,
                index: None,
                checksum_verification: checksum_verification.clone(),
                notes: Some(notes.clone()),
            },
            error,
        })?;

        output = Some(DeduplicateOutputInfo {
            path: config.out.to_string_lossy().into_owned(),
            written: true,
            overwritten,
        });

        if let LoadedInput::Bam { index_before, .. } = &scan.input {
            index = Some(DeduplicateIndexInfo {
                present_before: index_before.is_some(),
                valid_after: false,
                reindex_requested: config.reindex,
                reindexed: false,
                kind: index_before.as_ref().map(|resolved| resolved.kind),
            });

            if config.reindex {
                notes.push(
                    "Reindexing was requested, but BAM index writing remains deferred in this slice; any pre-existing index should be treated as invalid for the deduplicated output."
                        .to_string(),
                );
            }
        }

        if config.verify_checksum {
            checksum_verification = Some(compute_checksum_verification(config, &scan));
        }
    } else if config.verify_checksum {
        let mut info = checksum_verification.unwrap_or(DeduplicateChecksumVerificationInfo {
            requested: true,
            performed: false,
            mode: None,
            input_digest: None,
            output_digest: None,
            matched: None,
            notes: Vec::new(),
        });
        info.notes.push(
            "Checksum verification is deferred in dry-run mode because no output file was written."
                .to_string(),
        );
        checksum_verification = Some(info);
    } else {
        checksum_verification = None;
    }

    if !config.verify_checksum {
        checksum_verification = None;
    }

    let payload = DeduplicatePayload {
        format: scan.format,
        mode: config.mode,
        identity_mode: config.identity_mode,
        keep_policy: Some(config.keep_policy),
        execution: Some(DeduplicateExecutionInfo {
            dry_run: config.dry_run,
            modified,
        }),
        summary: Some(summary.clone()),
        ranges: Some(ranges.clone()),
        output,
        index,
        checksum_verification,
        notes: Some(notes.clone()),
    };

    if let Some(report_path) = &config.emit_removed_report {
        write_removed_report(config, &payload, &summary, &ranges, report_path).map_err(
            |error| DeduplicateFailure {
                payload: payload.clone(),
                error,
            },
        )?;
    }

    Ok(payload)
}

fn load_input(config: &DeduplicateConfig) -> Result<LoadedScan, AppError> {
    let probe = crate::formats::probe::probe_path(&config.input)?;

    match probe.detected_format {
        DetectedFormat::Fastq | DetectedFormat::FastqGz => {
            load_fastq(config, probe.detected_format)
        }
        DetectedFormat::Bam => load_bam(config),
        DetectedFormat::Unknown => Err(AppError::UnknownFormat {
            path: config.input.clone(),
        }),
        other => Err(AppError::UnsupportedInputForCommand {
            path: config.input.clone(),
            detail: format!(
                "deduplicate currently supports BAM, FASTQ, and FASTQ.GZ only; detected {other}."
            ),
        }),
    }
}

fn load_fastq(config: &DeduplicateConfig, format: DetectedFormat) -> Result<LoadedScan, AppError> {
    if config.identity_mode == DuplicationIdentityMode::QnameSeqQualRg {
        return Err(AppError::InvalidIdentityMode {
            path: config.input.clone(),
            detail:
                "Identity mode qname_seq_qual_rg requires BAM input because FASTQ records do not carry BAM read-group tags."
                    .to_string(),
        });
    }

    let record_limit = if config.dry_run && !config.full_scan {
        config.sample_records.max(1) as u64
    } else {
        u64::MAX
    };

    let mut reader = open_fastq_reader(&config.input)?;
    let mut records = Vec::new();
    let mut identities = Vec::new();
    let mut identity_lookup = HashMap::new();
    let mut records_examined = 0_u64;
    let mut reached_eof = false;

    while records_examined < record_limit {
        let record = match read_next_fastq_record(&mut reader, &config.input) {
            Ok(Some(record)) => record,
            Ok(None) => {
                reached_eof = true;
                break;
            }
            Err(AppError::InvalidFastq { detail, .. }) => {
                return Err(AppError::ParseUncertainty {
                    path: config.input.clone(),
                    detail,
                });
            }
            Err(error) => return Err(error),
        };

        let key = build_duplication_identity_key(
            config.identity_mode,
            &record.read_name,
            &record.sequence,
            Some(record.quality.as_str()),
            None,
        );
        let identity_id = intern_identity(&mut identity_lookup, &key);
        identities.push(identity_id);
        records.push(record);
        records_examined += 1;
    }

    Ok(LoadedScan {
        format,
        identities,
        records_examined,
        reached_eof,
        input: LoadedInput::Fastq { records },
    })
}

fn load_bam(config: &DeduplicateConfig) -> Result<LoadedScan, AppError> {
    let record_limit = if config.dry_run && !config.full_scan {
        config.sample_records.max(1) as u64
    } else {
        u64::MAX
    };
    let index_before = match resolve_index_for_bam(&config.input) {
        IndexResolution::Present(resolved) | IndexResolution::Unsupported(resolved) => {
            Some(resolved)
        }
        IndexResolution::NotFound => None,
    };

    let mut reader = BamReader::open(&config.input)?;
    let header =
        parse_bam_header_from_reader(&mut reader).map_err(|error| AppError::ParseUncertainty {
            path: config.input.clone(),
            detail: error.to_string(),
        })?;

    let mut records = Vec::new();
    let mut identities = Vec::new();
    let mut identity_lookup = HashMap::new();
    let mut records_examined = 0_u64;
    let mut reached_eof = false;

    while records_examined < record_limit {
        let layout = match read_next_record_layout(&mut reader) {
            Ok(Some(layout)) => layout,
            Ok(None) => {
                reached_eof = true;
                break;
            }
            Err(
                AppError::InvalidRecord { detail, .. } | AppError::TruncatedFile { detail, .. },
            ) => {
                return Err(AppError::ParseUncertainty {
                    path: config.input.clone(),
                    detail,
                });
            }
            Err(error) => return Err(error),
        };

        let sequence =
            decode_bam_sequence(&layout.sequence_bytes, layout.l_seq).map_err(|detail| {
                AppError::ParseUncertainty {
                    path: config.input.clone(),
                    detail,
                }
            })?;
        let quality = decode_bam_qualities(&layout.quality_bytes).map_err(|detail| {
            AppError::ParseUncertainty {
                path: config.input.clone(),
                detail,
            }
        })?;
        let read_group = if config.identity_mode == DuplicationIdentityMode::QnameSeqQualRg {
            extract_string_aux_tag(&layout.aux_bytes, *b"RG").map_err(|detail| {
                AppError::ParseUncertainty {
                    path: config.input.clone(),
                    detail,
                }
            })?
        } else {
            None
        };

        let key = build_duplication_identity_key(
            config.identity_mode,
            &layout.read_name,
            &sequence,
            Some(quality.as_str()),
            read_group.as_deref(),
        );
        let identity_id = intern_identity(&mut identity_lookup, &key);
        identities.push(identity_id);
        records.push(layout);
        records_examined += 1;
    }

    Ok(LoadedScan {
        format: DetectedFormat::Bam,
        identities,
        records_examined,
        reached_eof,
        input: LoadedInput::Bam {
            header,
            records,
            index_before,
        },
    })
}

fn build_ranges(config: &DeduplicateConfig, identities: &[usize]) -> Vec<DeduplicateRange> {
    detect_adjacent_duplicate_blocks(identities, config.min_block_size.max(1))
        .into_iter()
        .filter(|block| match config.mode {
            DeduplicateMode::ContiguousBlock => true,
            DeduplicateMode::WholeFileAppend => {
                block.finding_type == DuplicationFindingType::WholeFileAppendDuplicate
            }
            DeduplicateMode::GlobalExact => false,
        })
        .map(|block| {
            let first = DuplicateRange {
                start: block.first_start as u64 + 1,
                end: (block.first_start + block.block_len) as u64,
            };
            let second = DuplicateRange {
                start: (block.first_start + block.block_len) as u64 + 1,
                end: (block.first_start + block.block_len * 2) as u64,
            };
            let (keep_range, remove_range) = match config.keep_policy {
                DeduplicateKeepPolicy::First => (first, second),
                DeduplicateKeepPolicy::Last => (second, first),
            };

            DeduplicateRange {
                range_type: match block.finding_type {
                    DuplicationFindingType::WholeFileAppendDuplicate => {
                        DeduplicationRangeType::WholeFileAppendDuplicate
                    }
                    _ => DeduplicationRangeType::ContiguousBlockDuplicate,
                },
                keep_range,
                remove_range,
            }
        })
        .collect()
}

fn write_output(
    config: &DeduplicateConfig,
    scan: &LoadedScan,
    ranges: &[DeduplicateRange],
) -> Result<(), AppError> {
    if config.out.exists() && !config.force {
        return Err(AppError::OutputExists {
            path: config.out.clone(),
        });
    }

    let temp_output = temp_output_path(&config.out)?;
    let remove_mask = build_remove_mask(scan.records_examined as usize, ranges);

    let write_result = match &scan.input {
        LoadedInput::Fastq { records } => {
            let retained = records
                .iter()
                .enumerate()
                .filter(|(index, _)| !remove_mask[*index])
                .map(|(_, record)| record.clone())
                .collect::<Vec<_>>();
            write_fastq_records(&temp_output, &retained)
        }
        LoadedInput::Bam {
            header, records, ..
        } => write_bam_records(&temp_output, header, records, &remove_mask),
    };

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_output);
        return Err(error);
    }

    if config.out.exists() {
        fs::remove_file(&config.out).map_err(|error| AppError::WriteError {
            path: config.out.clone(),
            message: error.to_string(),
        })?;
    }
    fs::rename(&temp_output, &config.out).map_err(|error| AppError::WriteError {
        path: config.out.clone(),
        message: error.to_string(),
    })?;

    Ok(())
}

fn write_bam_records(
    path: &Path,
    header: &HeaderPayload,
    records: &[RecordLayout],
    remove_mask: &[bool],
) -> Result<(), AppError> {
    let header_bytes =
        serialize_bam_header_payload(&header.header.raw_header_text, &header.header.references);
    let mut writer = BgzfWriter::create(path)?;
    writer.write_all(&header_bytes)?;

    for (index, record) in records.iter().enumerate() {
        if remove_mask.get(index).copied().unwrap_or(false) {
            continue;
        }
        writer.write_all(&serialize_record_layout(record))?;
    }

    writer.finish()
}

fn build_remove_mask(total_records: usize, ranges: &[DeduplicateRange]) -> Vec<bool> {
    let mut mask = vec![false; total_records];
    for range in ranges {
        let start = range.remove_range.start.saturating_sub(1) as usize;
        let end = range.remove_range.end as usize;
        for slot in mask.iter_mut().take(end).skip(start) {
            *slot = true;
        }
    }
    mask
}

fn compute_checksum_verification(
    config: &DeduplicateConfig,
    scan: &LoadedScan,
) -> DeduplicateChecksumVerificationInfo {
    if config.dry_run {
        return DeduplicateChecksumVerificationInfo {
            requested: true,
            performed: false,
            mode: None,
            input_digest: None,
            output_digest: None,
            matched: None,
            notes: vec![
                "Checksum verification is deferred in dry-run mode because no output file was written."
                    .to_string(),
            ],
        };
    }

    if scan.format != DetectedFormat::Bam {
        return DeduplicateChecksumVerificationInfo {
            requested: true,
            performed: false,
            mode: None,
            input_digest: None,
            output_digest: None,
            matched: None,
            notes: vec!["Checksum verification is currently BAM-only in this slice.".to_string()],
        };
    }

    let options = ChecksumOptions {
        mode: ChecksumMode::CanonicalRecordOrder,
        algorithm: ChecksumAlgorithm::Sha256,
        include_header: false,
        excluded_tags: Default::default(),
        excluded_tag_strings: Vec::new(),
        filters: ChecksumFilters {
            only_primary: false,
            mapped_only: false,
        },
    };

    let input_digest = compute_checksums(&config.input, &options)
        .ok()
        .and_then(|payload| extract_digest(payload, ChecksumMode::CanonicalRecordOrder));
    let output_digest = compute_checksums(&config.out, &options)
        .ok()
        .and_then(|payload| extract_digest(payload, ChecksumMode::CanonicalRecordOrder));
    let performed = input_digest.is_some() && output_digest.is_some();

    let mut notes = vec![
        "Checksum comparison in deduplication mode is descriptive and does not imply equality because records may have been intentionally removed."
            .to_string(),
    ];
    if !performed {
        notes.push(
            "One or both canonical BAM digests could not be computed reliably in this slice."
                .to_string(),
        );
    }

    DeduplicateChecksumVerificationInfo {
        requested: true,
        performed,
        mode: Some(ChecksumMode::CanonicalRecordOrder),
        input_digest,
        output_digest,
        matched: None,
        notes,
    }
}

fn write_removed_report(
    config: &DeduplicateConfig,
    payload: &DeduplicatePayload,
    summary: &DeduplicateSummary,
    ranges: &[DeduplicateRange],
    report_path: &Path,
) -> Result<(), AppError> {
    if report_path.exists() && !config.force {
        return Err(AppError::OutputExists {
            path: report_path.to_path_buf(),
        });
    }

    let report = RemovedReport {
        input_path: config.input.to_string_lossy().into_owned(),
        output_path: payload.output.as_ref().map(|output| output.path.clone()),
        format: payload.format,
        mode: config.mode,
        identity_mode: config.identity_mode,
        keep_policy: config.keep_policy,
        dry_run: config.dry_run,
        summary: summary.clone(),
        ranges: ranges.to_vec(),
    };
    let body = if config.json_pretty {
        serde_json::to_string_pretty(&report)
    } else {
        serde_json::to_string(&report)
    }
    .map_err(|error| AppError::WriteError {
        path: report_path.to_path_buf(),
        message: error.to_string(),
    })?;
    fs::write(report_path, body).map_err(|error| AppError::WriteError {
        path: report_path.to_path_buf(),
        message: error.to_string(),
    })?;

    Ok(())
}

fn build_notes(config: &DeduplicateConfig, scan: &LoadedScan, modified: bool) -> Vec<String> {
    let mut notes = vec![
        "This command remediates collection duplication and operator-error duplication; it is not molecular duplicate marking.".to_string(),
        "BAM duplicate flags are not used as the primary deduplication basis for this command.".to_string(),
        format!(
            "Identity comparisons used {} for {} input.",
            identity_mode_label(config.identity_mode),
            scan.format
        ),
    ];

    if config.dry_run {
        notes.push("Dry run only. No file modifications were made.".to_string());
    } else if modified {
        notes.push("Contiguous duplicate range removal was applied conservatively according to the selected keep policy.".to_string());
    } else {
        notes.push(
            "No suspicious duplicated blocks were detected under the selected mode and identity policy."
                .to_string(),
        );
    }

    if matches!(scan.format, DetectedFormat::Fastq | DetectedFormat::FastqGz) {
        notes.push(
            "FASTQ output compression is inferred from the output filename extension; .gz writes gzip-compressed FASTQ and other extensions write plain FASTQ."
                .to_string(),
        );
    }

    if config.dry_run && !scan.reached_eof {
        notes.push(
            "The dry-run scan stopped at the bounded record limit before EOF, so the removal plan covers only the examined records."
                .to_string(),
        );
    } else if !config.dry_run && !scan.reached_eof {
        notes.push(
            "Applied deduplication forced a full-input scan even though bounded dry-run options were requested."
                .to_string(),
        );
    }

    notes
}

fn temp_output_path(out: &Path) -> Result<PathBuf, AppError> {
    let parent = out
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let file_name = out
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::WriteError {
            path: out.to_path_buf(),
            message: "Output path did not contain a valid UTF-8 filename.".to_string(),
        })?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    Ok(parent.join(format!(".tmp-{nonce}-{file_name}")))
}

fn intern_identity(identity_lookup: &mut HashMap<String, usize>, key: &str) -> usize {
    if let Some(existing) = identity_lookup.get(key) {
        *existing
    } else {
        let next = identity_lookup.len();
        identity_lookup.insert(key.to_string(), next);
        next
    }
}

fn detected_format_from_error(config: &DeduplicateConfig, error: &AppError) -> DetectedFormat {
    match error {
        AppError::InvalidFastq { .. } => DetectedFormat::Fastq,
        AppError::InvalidBam { .. }
        | AppError::InvalidHeader { .. }
        | AppError::InvalidRecord { .. }
        | AppError::NotBam { .. } => DetectedFormat::Bam,
        _ => crate::formats::probe::probe_path(&config.input)
            .map(|probe| probe.detected_format)
            .unwrap_or(DetectedFormat::Unknown),
    }
}

fn base_payload(
    format: DetectedFormat,
    mode: DeduplicateMode,
    identity_mode: DuplicationIdentityMode,
    keep_policy: DeduplicateKeepPolicy,
) -> DeduplicatePayload {
    DeduplicatePayload {
        format,
        mode,
        identity_mode,
        keep_policy: Some(keep_policy),
        execution: None,
        summary: None,
        ranges: None,
        output: None,
        index: None,
        checksum_verification: None,
        notes: None,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{
        bam::{
            header::{
                ReferenceHeaderFields, ReferenceRecord, parse_bam_header_from_reader,
                serialize_bam_header_payload,
            },
            reader::BamReader,
            records::{
                RecordLayout, encode_bam_qualities, encode_bam_sequence, read_next_record_layout,
            },
            write::{BgzfWriter, serialize_record_layout},
        },
        forensics::duplication::DuplicationIdentityMode,
    };

    use super::{DeduplicateConfig, DeduplicateKeepPolicy, DeduplicateMode, execute};

    #[test]
    fn dry_run_detects_whole_file_append_in_fastq() {
        let input = std::env::temp_dir().join(format!(
            "bamana-deduplicate-fastq-in-{}.fastq",
            std::process::id()
        ));
        let output = std::env::temp_dir().join(format!(
            "bamana-deduplicate-fastq-out-{}.fastq",
            std::process::id()
        ));
        fs::write(
            &input,
            "@r1 a\nACGT\n+\n!!!!\n@r2 b\nTGCA\n+\n####\n@r1 a\nACGT\n+\n!!!!\n@r2 b\nTGCA\n+\n####\n",
        )
        .expect("fixture should write");

        let payload = execute(&base_config(&input, &output, true)).expect("dry run should succeed");
        fs::remove_file(input).expect("fixture should be removable");

        assert!(
            payload
                .execution
                .as_ref()
                .is_some_and(|execution| execution.dry_run)
        );
        assert_eq!(
            payload
                .summary
                .as_ref()
                .map(|summary| summary.records_marked_for_removal),
            Some(Some(2))
        );
        assert_eq!(payload.ranges.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn applied_fastq_removes_duplicate_block() {
        let input = std::env::temp_dir().join(format!(
            "bamana-deduplicate-fastq-apply-in-{}.fastq",
            std::process::id()
        ));
        let output = std::env::temp_dir().join(format!(
            "bamana-deduplicate-fastq-apply-out-{}.fastq",
            std::process::id()
        ));
        fs::write(
            &input,
            "@r1 a\nACGT\n+\n!!!!\n@r2 b\nTGCA\n+\n####\n@r1 a\nACGT\n+\n!!!!\n@r2 b\nTGCA\n+\n####\n",
        )
        .expect("fixture should write");

        let payload =
            execute(&base_config(&input, &output, false)).expect("applied dedup should succeed");
        let contents = fs::read_to_string(&output).expect("output should be readable");
        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("output should be removable");

        assert!(
            payload
                .execution
                .as_ref()
                .is_some_and(|execution| !execution.dry_run)
        );
        assert_eq!(
            payload
                .summary
                .as_ref()
                .and_then(|summary| summary.records_removed),
            Some(2)
        );
        assert_eq!(contents, "@r1 a\nACGT\n+\n!!!!\n@r2 b\nTGCA\n+\n####\n");
    }

    #[test]
    fn applied_bam_removes_duplicate_block() {
        let input = std::env::temp_dir().join(format!(
            "bamana-deduplicate-bam-in-{}.bam",
            std::process::id()
        ));
        let output = std::env::temp_dir().join(format!(
            "bamana-deduplicate-bam-out-{}.bam",
            std::process::id()
        ));
        write_test_bam(
            &input,
            vec![
                build_test_record("r1", "ACGT", "!!!!", Some("rg1")),
                build_test_record("r2", "TGCA", "####", Some("rg1")),
                build_test_record("r1", "ACGT", "!!!!", Some("rg1")),
                build_test_record("r2", "TGCA", "####", Some("rg1")),
            ],
        );

        let payload =
            execute(&base_config(&input, &output, false)).expect("applied dedup should succeed");

        let mut reader = BamReader::open(&output).expect("output should reopen");
        let _header = parse_bam_header_from_reader(&mut reader).expect("header should parse");
        let mut retained = 0_u64;
        while read_next_record_layout(&mut reader)
            .expect("record scan should succeed")
            .is_some()
        {
            retained += 1;
        }

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("output should be removable");

        assert_eq!(
            payload
                .summary
                .as_ref()
                .and_then(|summary| summary.records_removed),
            Some(2)
        );
        assert_eq!(retained, 2);
    }

    fn base_config(input: &PathBuf, out: &PathBuf, dry_run: bool) -> DeduplicateConfig {
        DeduplicateConfig {
            input: input.clone(),
            out: out.clone(),
            mode: DeduplicateMode::ContiguousBlock,
            identity_mode: DuplicationIdentityMode::QnameSeqQual,
            keep_policy: DeduplicateKeepPolicy::First,
            dry_run,
            force: true,
            min_block_size: 2,
            verify_checksum: false,
            emit_removed_report: None,
            sample_records: 100,
            full_scan: true,
            reindex: false,
            json_pretty: true,
        }
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
