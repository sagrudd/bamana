use std::path::{Path, PathBuf};

use crate::{
    bam::annotate_rg::{
        AnnotateRgChecksumVerificationInfo, AnnotateRgConfig, AnnotateRgExecutionInfo,
        AnnotateRgHeaderInfo, AnnotateRgHeaderPolicy, AnnotateRgIndexInfo, AnnotateRgMode,
        AnnotateRgOutputInfo, AnnotateRgPayload, AnnotateRgRecordSummary,
        AnnotateRgRequestInfo, execute as execute_annotate_rg, preview as preview_annotate_rg,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct AnnotateRgRequest {
    pub bam: PathBuf,
    pub rg_id: String,
    pub out: Option<PathBuf>,
    pub only_missing: bool,
    pub replace_existing: bool,
    pub fail_on_conflict: bool,
    pub require_header_rg: bool,
    pub create_header_rg: bool,
    pub add_header_rg: Option<String>,
    pub set_header_rg: Option<String>,
    pub reindex: bool,
    pub verify_checksum: bool,
    pub threads: usize,
    pub force: bool,
    pub dry_run: bool,
}

pub fn run(request: AnnotateRgRequest) -> CommandResponse<AnnotateRgPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("annotate_rg", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "annotate_rg",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "annotate_rg",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "annotate_rg",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    if request.rg_id.is_empty()
        || request.rg_id.contains('\t')
        || request.rg_id.contains('\n')
        || request.rg_id.contains('\r')
    {
        return CommandResponse::failure_with_data(
            "annotate_rg",
            Some(request.bam.as_path()),
            base_payload(
                &request,
                AnnotateRgMode::FailOnConflict,
                AnnotateRgHeaderPolicy::RequireExisting,
            ),
            AppError::InvalidRgRequest {
                path: request.bam.clone(),
                detail: "--rg-id must be non-empty and may not contain tabs or newlines."
                    .to_string(),
            },
        );
    }

    let record_mode = match resolve_record_mode(&request) {
        Ok(mode) => mode,
        Err(error) => {
            return CommandResponse::failure_with_data(
                "annotate_rg",
                Some(request.bam.as_path()),
                base_payload(
                    &request,
                    AnnotateRgMode::FailOnConflict,
                    AnnotateRgHeaderPolicy::RequireExisting,
                ),
                error,
            );
        }
    };

    let header_policy = match resolve_header_policy(&request) {
        Ok(policy) => policy,
        Err(error) => {
            return CommandResponse::failure_with_data(
                "annotate_rg",
                Some(request.bam.as_path()),
                base_payload(
                    &request,
                    record_mode,
                    AnnotateRgHeaderPolicy::RequireExisting,
                ),
                error,
            );
        }
    };

    let config = AnnotateRgConfig {
        input_path: request.bam.clone(),
        output_path: request.out.clone(),
        rg_id: request.rg_id.clone(),
        record_mode,
        header_policy,
        add_header_rg: request.add_header_rg.clone(),
        set_header_rg: request.set_header_rg.clone(),
        dry_run: request.dry_run,
        force: request.force,
        reindex: request.reindex,
        verify_checksum: request.verify_checksum,
        threads: request.threads,
    };

    let payload = preview_annotate_rg(&config)
        .ok()
        .or_else(|| base_payload(&request, record_mode, header_policy));

    match execute_annotate_rg(&config) {
        Ok(result) => {
            CommandResponse::success("annotate_rg", Some(request.bam.as_path()), result.payload)
        }
        Err(error) => CommandResponse::failure_with_data(
            "annotate_rg",
            Some(request.bam.as_path()),
            payload,
            error,
        ),
    }
}

fn resolve_record_mode(request: &AnnotateRgRequest) -> Result<AnnotateRgMode, AppError> {
    let selected = request.only_missing as u8
        + request.replace_existing as u8
        + request.fail_on_conflict as u8;
    if selected > 1 {
        return Err(AppError::InvalidRgRequest {
            path: request.bam.clone(),
            detail:
                "Choose exactly one of --only-missing, --replace-existing, or --fail-on-conflict."
                    .to_string(),
        });
    }
    Ok(if request.only_missing {
        AnnotateRgMode::OnlyMissing
    } else if request.replace_existing {
        AnnotateRgMode::ReplaceExisting
    } else {
        AnnotateRgMode::FailOnConflict
    })
}

fn resolve_header_policy(request: &AnnotateRgRequest) -> Result<AnnotateRgHeaderPolicy, AppError> {
    let selected = request.require_header_rg as u8
        + request.create_header_rg as u8
        + request.add_header_rg.is_some() as u8
        + request.set_header_rg.is_some() as u8;
    if selected > 1 {
        return Err(AppError::InvalidRgRequest {
            path: request.bam.clone(),
            detail: "Choose at most one of --require-header-rg, --create-header-rg, --add-header-rg, or --set-header-rg.".to_string(),
        });
    }
    Ok(if request.add_header_rg.is_some() {
        AnnotateRgHeaderPolicy::AddHeaderRg
    } else if request.set_header_rg.is_some() {
        AnnotateRgHeaderPolicy::SetHeaderRg
    } else if request.create_header_rg {
        AnnotateRgHeaderPolicy::CreateIfMissing
    } else {
        AnnotateRgHeaderPolicy::RequireExisting
    })
}

fn base_payload(
    request: &AnnotateRgRequest,
    record_mode: AnnotateRgMode,
    header_policy: AnnotateRgHeaderPolicy,
) -> Option<AnnotateRgPayload> {
    Some(AnnotateRgPayload {
        format: "BAM",
        request: AnnotateRgRequestInfo {
            rg_id: request.rg_id.clone(),
            record_mode,
            header_policy,
        },
        execution: AnnotateRgExecutionInfo {
            mode_used: None,
            modified: false,
            dry_run: request.dry_run,
        },
        records: AnnotateRgRecordSummary {
            records_examined: None,
            records_annotated: None,
            records_already_matching: None,
            records_missing_rg_before: None,
            records_conflicting_before: None,
        },
        header: AnnotateRgHeaderInfo {
            rg_present_before: false,
            rg_present_after: false,
            header_modified: false,
        },
        output: AnnotateRgOutputInfo {
            path: request
                .out
                .clone()
                .unwrap_or_else(|| default_output_path(&request.bam))
                .to_string_lossy()
                .into_owned(),
            written: false,
            overwritten: None,
        },
        index: AnnotateRgIndexInfo {
            present_before: false,
            valid_after: if request.dry_run { None } else { Some(false) },
            reindex_requested: request.reindex,
            reindexed: false,
            kind: None,
        },
        checksum_verification: AnnotateRgChecksumVerificationInfo {
            requested: request.verify_checksum,
            performed: false,
            mode: None,
            excluded_tags: if request.verify_checksum {
                vec!["RG".to_string()]
            } else {
                Vec::new()
            },
            input_digest: None,
            output_digest: None,
            r#match: None,
        },
        notes: vec![
            "Per-record RG tags were modified or planned.".to_string(),
            "This command is distinct from header-only reheader and may rewrite every BAM alignment record.".to_string(),
        ],
    })
}

fn default_output_path(input: &Path) -> PathBuf {
    let input_str = input.to_string_lossy();
    if let Some(stripped) = input_str.strip_suffix(".bam") {
        PathBuf::from(format!("{stripped}.annotated_rg.bam"))
    } else {
        PathBuf::from(format!("{input_str}.annotated_rg.bam"))
    }
}
