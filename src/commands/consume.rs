use std::{collections::BTreeMap, path::PathBuf};

use serde::Serialize;

use crate::{
    error::AppError,
    formats::probe::DetectedFormat,
    ingest::{
        consume::{
            ConsumeMode, ConsumePlatform, ConsumeSortOrder, InputSemanticClass,
            classify_input_format, header_strategy_for_mode, mapped_state_for_mode,
        },
        discovery::{DiscoveryOptions, discover_requested_paths, format_counts},
    },
    json::CommandResponse,
};

#[derive(Debug)]
pub struct ConsumeRequest {
    pub input: Vec<PathBuf>,
    pub out: PathBuf,
    pub mode: ConsumeMode,
    pub recursive: bool,
    pub threads: usize,
    pub force: bool,
    pub sort: ConsumeSortOrder,
    pub create_index: bool,
    pub verify_checksum: bool,
    pub dry_run: bool,
    pub sample: Option<String>,
    pub read_group: Option<String>,
    pub platform: Option<ConsumePlatform>,
    pub include_glob: Vec<String>,
    pub exclude_glob: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConsumePayload {
    pub format: &'static str,
    pub mode: ConsumeMode,
    pub dry_run: bool,
    pub inputs: ConsumeInputRequest,
    pub discovery: ConsumeDiscoverySummary,
    pub output: ConsumeOutputInfo,
    pub header: ConsumeHeaderPolicyInfo,
    pub index: ConsumeIndexInfo,
    pub checksum_verification: ConsumeChecksumVerificationInfo,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConsumeInputRequest {
    pub requested_paths: Vec<String>,
    pub directories_scanned: usize,
    pub files_discovered: usize,
    pub files_consumed: usize,
    pub files_skipped: usize,
    pub files_rejected: usize,
}

#[derive(Debug, Serialize)]
pub struct ConsumeDiscoverySummary {
    pub recursive: bool,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub formats_detected: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub consumed_files: Vec<ConsumeInputFileInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped_files: Vec<ConsumeInputFileInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rejected_files: Vec<ConsumeInputFileInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsumeInputFileInfo {
    pub path: String,
    pub detected_format: String,
    pub consumed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConsumeOutputInfo {
    pub path: String,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_written: Option<u64>,
    pub sort_order: ConsumeSortOrder,
    pub mapped_state: String,
}

#[derive(Debug, Serialize)]
pub struct ConsumeHeaderPolicyInfo {
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_compatibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<ConsumePlatform>,
}

#[derive(Debug, Serialize)]
pub struct ConsumeIndexInfo {
    pub requested: bool,
    pub created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ConsumeChecksumVerificationInfo {
    pub requested: bool,
    pub performed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<bool>,
}

pub fn run(request: ConsumeRequest) -> CommandResponse<ConsumePayload> {
    if request.input.iter().any(|path| path == &request.out) {
        return CommandResponse::failure_with_data(
            "consume",
            None,
            Some(base_payload(&request)),
            AppError::InvalidConsumeRequest {
                path: request.out.clone(),
                detail: "Output path collides with one of the requested input paths.".to_string(),
            },
        );
    }

    let discovery = match discover_requested_paths(
        &request.input,
        &DiscoveryOptions {
            recursive: request.recursive,
        },
    ) {
        Ok(discovery) => discovery,
        Err(error) => {
            return CommandResponse::failure_with_data(
                "consume",
                None,
                Some(base_payload(&request)),
                error,
            );
        }
    };

    let mut payload = base_payload(&request);
    payload.inputs.directories_scanned = discovery.directories_scanned;
    payload.inputs.files_discovered = discovery.candidate_files.len();
    payload.discovery.formats_detected = format_counts(&discovery.discovered_files);
    payload
        .discovery
        .skipped_files
        .extend(
            discovery
                .skipped_entries
                .into_iter()
                .map(|entry| ConsumeInputFileInfo {
                    path: entry.path,
                    detected_format: entry.detected_format.to_string(),
                    consumed: entry.consumed,
                    reason: Some(entry.reason),
                }),
        );

    let mut alignment_files = Vec::new();
    let mut raw_files = Vec::new();
    let mut saw_cram = false;

    for file in discovery.discovered_files {
        if file.detected_format == DetectedFormat::Cram {
            saw_cram = true;
        }
        let info = ConsumeInputFileInfo {
            path: file.path.to_string_lossy().into_owned(),
            detected_format: file.detected_format.to_string(),
            consumed: false,
            reason: None,
        };

        match classify_input_format(file.detected_format) {
            InputSemanticClass::Alignment => alignment_files.push(info),
            InputSemanticClass::RawRead => raw_files.push(info),
            InputSemanticClass::Unsupported => {
                let mut skipped = info;
                skipped.reason = Some("unsupported_input_format".to_string());
                payload.discovery.skipped_files.push(skipped);
            }
        }
    }

    if !request.include_glob.is_empty() || !request.exclude_glob.is_empty() {
        return CommandResponse::failure_with_data(
            "consume",
            None,
            Some(payload),
            AppError::Unimplemented {
                path: request.out.clone(),
                detail: "Include/exclude glob filtering is planned for consume but not implemented in this slice.".to_string(),
            },
        );
    }

    if matches!(request.mode, ConsumeMode::MixedAllow) {
        return CommandResponse::failure_with_data(
            "consume",
            None,
            Some(payload),
            AppError::Unimplemented {
                path: request.out.clone(),
                detail: "Mixed alignment-bearing and raw-read ingestion is not implemented in this slice.".to_string(),
            },
        );
    }

    if alignment_files.is_empty() && raw_files.is_empty() {
        if saw_cram && matches!(request.mode, ConsumeMode::Alignment) {
            return CommandResponse::failure_with_data(
                "consume",
                None,
                Some(payload),
                AppError::ReferenceRequired {
                    path: request.out.clone(),
                    detail: "CRAM ingestion is staged behind an explicit reference policy and is not implemented in this slice.".to_string(),
                },
            );
        }

        return CommandResponse::failure_with_data(
            "consume",
            None,
            Some(payload),
            AppError::UnsupportedInputFormat {
                path: request.out.clone(),
                format: "No supported BAM, SAM, FASTQ, or FASTQ.GZ inputs were discovered."
                    .to_string(),
            },
        );
    }

    if !alignment_files.is_empty() && !raw_files.is_empty() {
        let detail = format!(
            "Detected {} alignment-bearing input(s) and {} raw-read input(s) in the same request.",
            alignment_files.len(),
            raw_files.len()
        );

        payload
            .discovery
            .rejected_files
            .extend(
                alignment_files
                    .into_iter()
                    .chain(raw_files)
                    .map(|mut file| {
                        file.reason = Some("mixed_input_modes_not_allowed".to_string());
                        file
                    }),
            );
        refresh_counts(&mut payload);

        return CommandResponse::failure_with_data(
            "consume",
            None,
            Some(payload),
            AppError::MixedInputModesNotAllowed {
                path: request.out.clone(),
                detail,
            },
        );
    }

    match request.mode {
        ConsumeMode::Alignment => {
            if alignment_files.is_empty() {
                payload
                    .discovery
                    .rejected_files
                    .extend(raw_files.into_iter().map(|mut file| {
                        file.reason = Some("unsupported_input_format".to_string());
                        file
                    }));
                refresh_counts(&mut payload);
                return CommandResponse::failure_with_data(
                    "consume",
                    None,
                    Some(payload),
                    AppError::UnsupportedInputFormat {
                        path: request.out.clone(),
                        format: "Alignment mode supports BAM and SAM inputs only.".to_string(),
                    },
                );
            }

            payload.discovery.consumed_files = alignment_files
                .into_iter()
                .map(|mut file| {
                    file.consumed = true;
                    file
                })
                .collect();
            payload.header.reference_compatibility = Some("planned_strict_match".to_string());
            payload
                .notes
                .push("Alignment mode consumes BAM and SAM inputs and preserves alignments where present.".to_string());
        }
        ConsumeMode::Unmapped => {
            if raw_files.is_empty() {
                payload
                    .discovery
                    .rejected_files
                    .extend(alignment_files.into_iter().map(|mut file| {
                        file.reason = Some("unsupported_input_format".to_string());
                        file
                    }));
                refresh_counts(&mut payload);
                return CommandResponse::failure_with_data(
                    "consume",
                    None,
                    Some(payload),
                    AppError::UnsupportedInputFormat {
                        path: request.out.clone(),
                        format: "Unmapped mode supports FASTQ and FASTQ.GZ inputs only."
                            .to_string(),
                    },
                );
            }

            payload.discovery.consumed_files = raw_files
                .into_iter()
                .map(|mut file| {
                    file.consumed = true;
                    file
                })
                .collect();
            payload
                .notes
                .push("FASTQ-like inputs are normalized into unmapped BAM records with no implied alignment.".to_string());
        }
        ConsumeMode::MixedAllow => unreachable!(),
    }

    refresh_counts(&mut payload);
    push_staged_notes(&request, &mut payload);

    if request.dry_run {
        payload.notes.push(
            "Dry-run mode performed deterministic discovery, classification, and policy enforcement without writing a BAM output."
                .to_string(),
        );
        return CommandResponse::success("consume", None, payload);
    }

    if request.out.exists() && !request.force {
        return CommandResponse::failure_with_data(
            "consume",
            None,
            Some(payload),
            AppError::OutputExists {
                path: request.out.clone(),
            },
        );
    }

    CommandResponse::failure_with_data(
        "consume",
        None,
        Some(payload),
        AppError::Unimplemented {
            path: request.out.clone(),
            detail: "Current consume support is limited to deterministic discovery, classification, policy enforcement, and dry-run planning. BAM writing and format normalization are staged but not implemented in this slice.".to_string(),
        },
    )
}

fn base_payload(request: &ConsumeRequest) -> ConsumePayload {
    ConsumePayload {
        format: "BAM",
        mode: request.mode,
        dry_run: request.dry_run,
        inputs: ConsumeInputRequest {
            requested_paths: request
                .input
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            directories_scanned: 0,
            files_discovered: 0,
            files_consumed: 0,
            files_skipped: 0,
            files_rejected: 0,
        },
        discovery: ConsumeDiscoverySummary {
            recursive: request.recursive,
            formats_detected: BTreeMap::new(),
            consumed_files: Vec::new(),
            skipped_files: Vec::new(),
            rejected_files: Vec::new(),
        },
        output: ConsumeOutputInfo {
            path: request.out.to_string_lossy().into_owned(),
            written: false,
            records_written: None,
            sort_order: request.sort,
            mapped_state: mapped_state_for_mode(request.mode).to_string(),
        },
        header: ConsumeHeaderPolicyInfo {
            strategy: header_strategy_for_mode(request.mode).to_string(),
            reference_compatibility: None,
            sample: request.sample.clone(),
            read_group: request.read_group.clone(),
            platform: request.platform,
        },
        index: ConsumeIndexInfo {
            requested: request.create_index,
            created: false,
            kind: None,
        },
        checksum_verification: ConsumeChecksumVerificationInfo {
            requested: request.verify_checksum,
            performed: false,
            mode: None,
            r#match: None,
        },
        notes: Vec::new(),
    }
}

fn refresh_counts(payload: &mut ConsumePayload) {
    payload.inputs.files_consumed = payload.discovery.consumed_files.len();
    payload.inputs.files_skipped = payload.discovery.skipped_files.len();
    payload.inputs.files_rejected = payload.discovery.rejected_files.len();
}

fn push_staged_notes(request: &ConsumeRequest, payload: &mut ConsumePayload) {
    payload.notes.push(
        "Directory traversal is lexical by normalized path string; directories are scanned top-level only unless --recursive is supplied, and symlinks are not followed in this slice."
            .to_string(),
    );

    if request.sort != ConsumeSortOrder::None {
        payload.notes.push(
            "Requested post-ingest sorting is recorded in the contract, but consume does not yet execute sort reuse in this slice.".to_string(),
        );
    }
    if request.create_index {
        payload.notes.push(
            "Index creation is only meaningful for coordinate-sorted BAM output and is deferred for consume in this slice.".to_string(),
        );
    }
    if request.verify_checksum {
        payload.notes.push(
            "Checksum verification is planned to reuse Bamana checksum modes after ingest, but it is not yet performed in this slice.".to_string(),
        );
    }
    if request.threads > 1 {
        payload.notes.push(
            "Thread-count acceptance is part of the consume contract; parallel ingestion is not yet implemented.".to_string(),
        );
    }
}
