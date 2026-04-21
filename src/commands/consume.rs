use std::{collections::BTreeMap, path::PathBuf};

use serde::Serialize;

use crate::{
    error::AppError,
    formats::probe::DetectedFormat,
    ingest::{
        consume::{
            ConsumeExecutionOptions, ConsumeMode, ConsumePlatform, ConsumeSortOrder,
            InputSemanticClass, classify_input_format, execute_consume, header_strategy_for_mode,
            mapped_state_for_mode, prepare_cram_context_for_consume,
        },
        cram::{ConsumeReferencePolicy, ConsumeReferenceSourceUsed},
        discovery::{
            DiscoveredFile, DiscoveryOptions, cleanup_staged_paths, discover_requested_paths,
            format_counts,
        },
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
    pub reference: Option<PathBuf>,
    pub reference_cache: Option<PathBuf>,
    pub reference_policy: ConsumeReferencePolicy,
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
    pub reference: ConsumeReferencePolicyInfo,
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
pub struct ConsumeReferencePolicyInfo {
    pub policy: ConsumeReferencePolicy,
    pub explicit_reference_provided: bool,
    pub reference_cache_provided: bool,
    pub cram_inputs_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_used: Option<ConsumeReferenceSourceUsed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode_without_external_reference: Option<bool>,
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
    let (response, staged_paths) = run_impl(&request);
    cleanup_staged_paths(&staged_paths);
    response
}

fn run_impl(request: &ConsumeRequest) -> (CommandResponse<ConsumePayload>, Vec<PathBuf>) {
    if request.input.iter().any(|path| path == &request.out) {
        return (
            CommandResponse::failure_with_data(
                "consume",
                None,
                Some(base_payload(request)),
                AppError::InvalidConsumeRequest {
                    path: request.out.clone(),
                    detail: "Output path collides with one of the requested input paths."
                        .to_string(),
                },
            ),
            Vec::new(),
        );
    }

    if !request.include_glob.is_empty() || !request.exclude_glob.is_empty() {
        return (
            CommandResponse::failure_with_data(
                "consume",
                None,
                Some(base_payload(request)),
                AppError::Unimplemented {
                    path: request.out.clone(),
                    detail: "Include/exclude glob filtering is planned for consume but not implemented in this slice.".to_string(),
                },
            ),
            Vec::new(),
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
            return (
                CommandResponse::failure_with_data(
                    "consume",
                    None,
                    Some(base_payload(request)),
                    error,
                ),
                Vec::new(),
            );
        }
    };
    let staged_paths = discovery.staged_paths.clone();

    let mut payload = base_payload(request);
    payload.inputs.directories_scanned = discovery.directories_scanned;
    payload.inputs.files_discovered = discovery.discovered_files.len();
    payload.discovery.formats_detected = format_counts(&discovery.discovered_files);
    payload.reference.cram_inputs_present = discovery
        .discovered_files
        .iter()
        .any(|file| matches!(file.detected_format, DetectedFormat::Cram));
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

    let classified = classify_files(&discovery.discovered_files, &mut payload);

    let active_files = match resolve_mode_files(&request, &mut payload, &classified) {
        Ok(files) => files,
        Err(error) => {
            return (
                CommandResponse::failure_with_data("consume", None, Some(payload), error),
                staged_paths,
            );
        }
    };

    if active_files
        .iter()
        .any(|file| matches!(file.detected_format, DetectedFormat::Cram))
    {
        match prepare_cram_context_for_consume(
            &request.out,
            request.reference_policy,
            request.reference.as_deref(),
            request.reference_cache.as_deref(),
            request.dry_run,
        ) {
            Ok(context) => {
                payload.reference.source_used = context.source_used_hint;
                payload.reference.decode_without_external_reference =
                    context.decode_without_external_reference_hint;
                if request.dry_run {
                    payload.notes.extend(context.notes);
                }
            }
            Err(error) => {
                return (
                    CommandResponse::failure_with_data("consume", None, Some(payload), error),
                    staged_paths,
                );
            }
        }
    }

    refresh_counts(&mut payload);
    push_stage1_notes(&request, &mut payload);

    if request.dry_run {
        payload.notes.push(
            "Dry-run mode performed deterministic discovery, classification, and policy enforcement without writing a BAM output."
                .to_string(),
        );
        return (
            CommandResponse::success("consume", None, payload),
            staged_paths,
        );
    }

    if request.create_index {
        let error = if request.sort != ConsumeSortOrder::Coordinate {
            AppError::InvalidConsumeRequest {
                path: request.out.clone(),
                detail:
                    "Index creation is only semantically valid for coordinate-sorted consume output."
                        .to_string(),
            }
        } else {
            AppError::Unimplemented {
                path: request.out.clone(),
                detail: "Index creation after consume is not implemented in this slice."
                    .to_string(),
            }
        };
        return (
            CommandResponse::failure_with_data("consume", None, Some(payload), error),
            staged_paths,
        );
    }

    if request.verify_checksum {
        return (
            CommandResponse::failure_with_data(
                "consume",
                None,
                Some(payload),
                AppError::Unimplemented {
                    path: request.out.clone(),
                    detail: "Checksum verification after consume is not implemented in this slice."
                        .to_string(),
                },
            ),
            staged_paths,
        );
    }

    let execution = match execute_consume(&ConsumeExecutionOptions {
        mode: request.mode,
        files: active_files,
        output_path: request.out.clone(),
        threads: request.threads,
        force: request.force,
        sort: request.sort,
        reference: request.reference.clone(),
        reference_cache: request.reference_cache.clone(),
        reference_policy: request.reference_policy,
        sample: request.sample.clone(),
        read_group: request.read_group.clone(),
        platform: request.platform,
    }) {
        Ok(execution) => execution,
        Err(error) => {
            return (
                CommandResponse::failure_with_data("consume", None, Some(payload), error),
                staged_paths,
            );
        }
    };

    payload.output.written = true;
    payload.output.records_written = Some(execution.records_written);
    payload.reference.source_used = execution.reference_source_used;
    payload.reference.decode_without_external_reference =
        execution.decode_without_external_reference;
    payload.header.strategy = execution.header_strategy;
    payload.header.reference_compatibility = execution.reference_compatibility;
    payload.notes.extend(execution.notes);
    if execution.overwritten {
        payload
            .notes
            .push("Existing output path was overwritten because --force was supplied.".to_string());
    }

    (
        CommandResponse::success("consume", None, payload),
        staged_paths,
    )
}

struct ClassifiedFiles {
    alignment: Vec<DiscoveredFile>,
    raw: Vec<DiscoveredFile>,
    unsupported: Vec<DiscoveredFile>,
}

fn classify_files(files: &[DiscoveredFile], payload: &mut ConsumePayload) -> ClassifiedFiles {
    let mut classified = ClassifiedFiles {
        alignment: Vec::new(),
        raw: Vec::new(),
        unsupported: Vec::new(),
    };

    for file in files {
        match classify_input_format(file.detected_format) {
            InputSemanticClass::Alignment => classified.alignment.push(file.clone()),
            InputSemanticClass::RawRead => classified.raw.push(file.clone()),
            InputSemanticClass::Unsupported => {
                payload.discovery.skipped_files.push(ConsumeInputFileInfo {
                    path: file.logical_path.to_string_lossy().into_owned(),
                    detected_format: file.detected_format.to_string(),
                    consumed: false,
                    reason: Some("unsupported_input_format".to_string()),
                });
                classified.unsupported.push(file.clone());
            }
        }
    }

    classified
}

fn resolve_mode_files(
    request: &ConsumeRequest,
    payload: &mut ConsumePayload,
    classified: &ClassifiedFiles,
) -> Result<Vec<DiscoveredFile>, AppError> {
    if classified.alignment.is_empty() && classified.raw.is_empty() {
        return Err(AppError::UnsupportedInputFormat {
            path: request.out.clone(),
            format: "No supported BAM, SAM, CRAM, FASTQ, or FASTQ.GZ inputs were discovered."
                .to_string(),
        });
    }

    if !classified.alignment.is_empty() && !classified.raw.is_empty() {
        payload.discovery.rejected_files.extend(
            classified
                .alignment
                .iter()
                .chain(&classified.raw)
                .map(|file| ConsumeInputFileInfo {
                    path: file.logical_path.to_string_lossy().into_owned(),
                    detected_format: file.detected_format.to_string(),
                    consumed: false,
                    reason: Some("mixed_input_modes_not_allowed".to_string()),
                }),
        );
        refresh_counts(payload);

        return Err(AppError::MixedInputModesNotAllowed {
            path: request.out.clone(),
            detail: format!(
                "Detected {} alignment-bearing input(s) and {} raw-read input(s) in the same request.",
                classified.alignment.len(),
                classified.raw.len()
            ),
        });
    }

    match request.mode {
        ConsumeMode::Alignment => {
            if classified.alignment.is_empty() {
                payload
                    .discovery
                    .rejected_files
                    .extend(classified.raw.iter().map(|file| ConsumeInputFileInfo {
                        path: file.logical_path.to_string_lossy().into_owned(),
                        detected_format: file.detected_format.to_string(),
                        consumed: false,
                        reason: Some("unsupported_input_for_mode".to_string()),
                    }));
                refresh_counts(payload);
                return Err(AppError::UnsupportedInputForMode {
                    path: request.out.clone(),
                    detail: "Alignment mode supports BAM, SAM, and CRAM inputs only.".to_string(),
                });
            }

            payload.discovery.consumed_files =
                classified.alignment.iter().map(as_consumed_file).collect();
            payload.header.reference_compatibility =
                Some("pending_compatibility_check".to_string());
            Ok(classified.alignment.clone())
        }
        ConsumeMode::Unmapped => {
            if classified.raw.is_empty() {
                payload
                    .discovery
                    .rejected_files
                    .extend(
                        classified
                            .alignment
                            .iter()
                            .map(|file| ConsumeInputFileInfo {
                                path: file.logical_path.to_string_lossy().into_owned(),
                                detected_format: file.detected_format.to_string(),
                                consumed: false,
                                reason: Some("unsupported_input_for_mode".to_string()),
                            }),
                    );
                refresh_counts(payload);
                return Err(AppError::UnsupportedInputForMode {
                    path: request.out.clone(),
                    detail:
                        "Unmapped mode supports FASTQ and FASTQ.GZ inputs only; BAM, SAM, and CRAM remain alignment-bearing inputs."
                            .to_string(),
                });
            }

            payload.discovery.consumed_files =
                classified.raw.iter().map(as_consumed_file).collect();
            Ok(classified.raw.clone())
        }
    }
}

fn as_consumed_file(file: &DiscoveredFile) -> ConsumeInputFileInfo {
    ConsumeInputFileInfo {
        path: file.logical_path.to_string_lossy().into_owned(),
        detected_format: file.detected_format.to_string(),
        consumed: true,
        reason: None,
    }
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
        reference: ConsumeReferencePolicyInfo {
            policy: request.reference_policy,
            explicit_reference_provided: request.reference.is_some(),
            reference_cache_provided: request.reference_cache.is_some(),
            cram_inputs_present: false,
            source_used: None,
            decode_without_external_reference: None,
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

fn push_stage1_notes(request: &ConsumeRequest, payload: &mut ConsumePayload) {
    payload.notes.push(
        "Requested file arguments are consumed in the order provided; directory contents are traversed lexically, directories are scanned top-level only unless --recursive is supplied, and symlinks are not followed in this slice."
            .to_string(),
    );

    if request.input.iter().any(|path| path == &PathBuf::from("-")) {
        payload.notes.push(
            "STDIN is staged to a temporary input for deterministic probing and unified compatibility checks; uncompressed SAM or FASTQ over stdin is the canonical streaming path in this slice."
                .to_string(),
        );
    }

    if request.mode == ConsumeMode::Alignment
        && (request.sample.is_some() || request.read_group.is_some() || request.platform.is_some())
    {
        payload.notes.push(
            "Sample/read-group/platform options are used only for synthetic unmapped headers and do not modify preserved alignment headers in alignment mode."
                .to_string(),
        );
    }
    if request.create_index {
        payload.notes.push(
            "Index creation is only meaningful for coordinate-sorted BAM output and remains deferred for consume in this slice.".to_string(),
        );
    }
    if request.verify_checksum {
        payload.notes.push(
            "Checksum verification is planned to reuse Bamana checksum modes after ingest, but it is not yet performed in this slice.".to_string(),
        );
    }
    if payload.reference.cram_inputs_present {
        payload.notes.push(format!(
            "CRAM ingestion is governed by the {} reference policy; Bamana does not silently guess CRAM reference behavior.",
            request.reference_policy
        ));
    } else if request.reference.is_some() || request.reference_cache.is_some() {
        payload.notes.push(
            "Reference-related consume options were provided, but no CRAM inputs were discovered in this request."
                .to_string(),
        );
    }
    if request.threads == 1 {
        payload.notes.push(
            "Single-thread consume remains the deterministic path for otherwise parallelisable FASTQ.GZ imports."
                .to_string(),
        );
    }
}
