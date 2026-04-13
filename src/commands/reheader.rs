use std::path::PathBuf;

use crate::{
    bam::reheader::{
        ReheaderChecksumVerificationInfo, ReheaderConfig, ReheaderExecutionMode,
        ReheaderExecutionPlan, ReheaderExecutionResult, ReheaderIndexInfo,
        ReheaderMutationOperation, ReheaderOperationKind, ReheaderOutputInfo, ReheaderPayload,
        RequestedHeaderMutation, execute as execute_reheader, preview as preview_reheader,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct ReheaderRequest {
    pub bam: PathBuf,
    pub out: Option<PathBuf>,
    pub header: Option<PathBuf>,
    pub add_rg: Vec<String>,
    pub set_rg: Vec<String>,
    pub remove_rg: Vec<String>,
    pub set_sample: Option<String>,
    pub set_platform: Option<String>,
    pub target_rg: Option<String>,
    pub set_pg: Vec<String>,
    pub add_comment: Vec<String>,
    pub in_place: bool,
    pub rewrite_minimized: bool,
    pub safe_rewrite: bool,
    pub dry_run: bool,
    pub force: bool,
    pub reindex: bool,
    pub verify_checksum: bool,
}

pub fn run(request: ReheaderRequest) -> CommandResponse<ReheaderPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("reheader", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "reheader",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "reheader",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "reheader",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    if request.safe_rewrite && request.in_place {
        return CommandResponse::failure(
            "reheader",
            Some(request.bam.as_path()),
            AppError::UnsupportedReheaderMode {
                path: request.bam.clone(),
                detail: "--safe-rewrite cannot be combined with --in-place.".to_string(),
            },
        );
    }

    if request.safe_rewrite && request.rewrite_minimized {
        return CommandResponse::failure(
            "reheader",
            Some(request.bam.as_path()),
            AppError::UnsupportedReheaderMode {
                path: request.bam.clone(),
                detail: "--safe-rewrite cannot be combined with --rewrite-minimized.".to_string(),
            },
        );
    }

    let requested_mode = if request.safe_rewrite {
        ReheaderExecutionMode::SafeRewrite
    } else if request.in_place {
        ReheaderExecutionMode::InPlace
    } else {
        ReheaderExecutionMode::RewriteMinimized
    };
    let rewrite_fallback_permitted = request.in_place && request.rewrite_minimized;

    let config = ReheaderConfig {
        input_path: request.bam.clone(),
        output_path: request.out.clone(),
        requested_mode,
        rewrite_fallback_permitted,
        dry_run: request.dry_run,
        force: request.force,
        reindex: request.reindex,
        verify_checksum: request.verify_checksum,
        header_path: request.header.clone(),
        add_rgs: request.add_rg.clone(),
        set_rgs: request.set_rg.clone(),
        remove_rgs: request.remove_rg.clone(),
        set_sample: request.set_sample.clone(),
        set_platform: request.set_platform.clone(),
        target_rg: request.target_rg.clone(),
        set_pgs: request.set_pg.clone(),
        add_comments: request.add_comment.clone(),
    };

    let preview = preview_reheader(&config);
    let payload = preview
        .ok()
        .or_else(|| base_payload(&request, requested_mode));

    match execute_reheader(&config) {
        Ok(result) => {
            CommandResponse::success("reheader", Some(request.bam.as_path()), result.payload)
        }
        Err(error) => CommandResponse::failure_with_data(
            "reheader",
            Some(request.bam.as_path()),
            payload,
            error,
        ),
    }
}

fn base_payload(
    request: &ReheaderRequest,
    requested_mode: ReheaderExecutionMode,
) -> Option<ReheaderPayload> {
    let operations = mutation_operations(request);
    if operations.is_empty() {
        return None;
    }

    Some(ReheaderPayload {
        format: "BAM",
        mutation: RequestedHeaderMutation {
            operations,
        },
        planning: ReheaderExecutionPlan {
            mode_requested: requested_mode,
            in_place_feasible: false,
            recommended_mode: if requested_mode == ReheaderExecutionMode::SafeRewrite {
                ReheaderExecutionMode::SafeRewrite
            } else {
                ReheaderExecutionMode::RewriteMinimized
            },
            reason: Some(
                "True BGZF header patching is not implemented safely in this slice, so true in-place execution is not currently proven safe."
                    .to_string(),
            ),
        },
        execution: ReheaderExecutionResult {
            mode_used: None,
            modified: false,
            dry_run: request.dry_run,
        },
        output: ReheaderOutputInfo {
            path: request
                .out
                .clone()
                .unwrap_or_else(|| default_output_path(&request.bam))
                .to_string_lossy()
                .into_owned(),
            written: false,
            overwritten: None,
        },
        index: ReheaderIndexInfo {
            present_before: false,
            valid_after: if request.dry_run { None } else { Some(false) },
            reindex_requested: request.reindex,
            reindexed: false,
            kind: None,
        },
        checksum_verification: ReheaderChecksumVerificationInfo {
            requested: request.verify_checksum,
            performed: false,
            mode: None,
            header_included: false,
            input_digest: None,
            output_digest: None,
            r#match: None,
        },
        notes: vec![
            "Header-only mutation was applied or planned.".to_string(),
            "This command modifies only BAM header metadata; it does not add, remove, or replace per-record RG:Z tags in alignment records.".to_string(),
        ],
    })
}

fn mutation_operations(request: &ReheaderRequest) -> Vec<ReheaderMutationOperation> {
    let mut operations = Vec::new();

    if let Some(header) = &request.header {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::ReplaceHeader,
            source: Some(header.to_string_lossy().into_owned()),
            target_rg: None,
            details: None,
            comment: None,
        });
    }

    for spec in &request.add_rg {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::AddRg,
            source: None,
            target_rg: extract_id(spec),
            details: Some(parse_kv_pairs(spec)),
            comment: None,
        });
    }

    for spec in &request.set_rg {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetRg,
            source: None,
            target_rg: extract_id(spec).or_else(|| request.target_rg.clone()),
            details: Some(parse_kv_pairs(spec)),
            comment: None,
        });
    }

    for id in &request.remove_rg {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::RemoveRg,
            source: None,
            target_rg: Some(id.clone()),
            details: None,
            comment: None,
        });
    }

    if let Some(sample) = &request.set_sample {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetSample,
            source: None,
            target_rg: request.target_rg.clone(),
            details: Some([("SM".to_string(), sample.clone())].into_iter().collect()),
            comment: None,
        });
    }

    if let Some(platform) = &request.set_platform {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetPlatform,
            source: None,
            target_rg: request.target_rg.clone(),
            details: Some([("PL".to_string(), platform.clone())].into_iter().collect()),
            comment: None,
        });
    }

    for spec in &request.set_pg {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetPg,
            source: None,
            target_rg: None,
            details: Some(parse_kv_pairs(spec)),
            comment: None,
        });
    }

    for comment in &request.add_comment {
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::AddComment,
            source: None,
            target_rg: None,
            details: None,
            comment: Some(comment.clone()),
        });
    }

    operations
}

fn parse_kv_pairs(raw: &str) -> std::collections::BTreeMap<String, String> {
    raw.split(',')
        .filter_map(|segment| {
            segment
                .split_once('=')
                .map(|(key, value)| (key.to_string(), value.to_string()))
        })
        .collect()
}

fn extract_id(raw: &str) -> Option<String> {
    let mut fields = parse_kv_pairs(raw);
    fields.remove("ID")
}

fn default_output_path(input: &std::path::Path) -> PathBuf {
    let input_str = input.to_string_lossy();
    if let Some(stripped) = input_str.strip_suffix(".bam") {
        PathBuf::from(format!("{stripped}.reheadered.bam"))
    } else {
        PathBuf::from(format!("{input_str}.reheadered.bam"))
    }
}
