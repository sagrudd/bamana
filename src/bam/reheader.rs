use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    bam::{
        checksum::{
            ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions, compute_checksums,
            extract_digest,
        },
        header::{
            BamHeaderView, ProgramRecord, ReadGroupRecord, parse_bam_header,
            parse_bam_header_from_reader, parse_sam_header_text_with_references,
            serialize_bam_header_payload, serialize_sam_header_text,
        },
        index::{IndexKind, discover_index_candidates},
        reader::BamReader,
        records::read_next_record_layout,
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReheaderExecutionMode {
    InPlace,
    RewriteMinimized,
    SafeRewrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReheaderOperationKind {
    ReplaceHeader,
    AddRg,
    SetRg,
    RemoveRg,
    SetSample,
    SetPlatform,
    SetPg,
    AddComment,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReheaderMutationOperation {
    pub operation: ReheaderOperationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_rg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestedHeaderMutation {
    pub operations: Vec<ReheaderMutationOperation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReheaderExecutionPlan {
    pub mode_requested: ReheaderExecutionMode,
    pub in_place_feasible: bool,
    pub recommended_mode: ReheaderExecutionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReheaderExecutionResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_used: Option<ReheaderExecutionMode>,
    pub modified: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReheaderOutputInfo {
    pub path: String,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwritten: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReheaderIndexInfo {
    pub present_before: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_after: Option<bool>,
    pub reindex_requested: bool,
    pub reindexed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<IndexKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReheaderChecksumVerificationInfo {
    pub requested: bool,
    pub performed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ChecksumMode>,
    pub header_included: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReheaderPayload {
    pub format: &'static str,
    pub mutation: RequestedHeaderMutation,
    pub planning: ReheaderExecutionPlan,
    pub execution: ReheaderExecutionResult,
    pub output: ReheaderOutputInfo,
    pub index: ReheaderIndexInfo,
    pub checksum_verification: ReheaderChecksumVerificationInfo,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReheaderConfig {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub requested_mode: ReheaderExecutionMode,
    pub rewrite_fallback_permitted: bool,
    pub dry_run: bool,
    pub force: bool,
    pub reindex: bool,
    pub verify_checksum: bool,
    pub header_path: Option<PathBuf>,
    pub add_rgs: Vec<String>,
    pub set_rgs: Vec<String>,
    pub remove_rgs: Vec<String>,
    pub set_sample: Option<String>,
    pub set_platform: Option<String>,
    pub target_rg: Option<String>,
    pub set_pgs: Vec<String>,
    pub add_comments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReheaderExecution {
    pub payload: ReheaderPayload,
}

pub fn preview(config: &ReheaderConfig) -> Result<ReheaderPayload, AppError> {
    let original_header = parse_bam_header(&config.input_path)?;
    let index_candidates = discover_index_candidates(&config.input_path, false);
    let present_before = !index_candidates.is_empty();
    let index_kind = index_candidates.first().map(|candidate| candidate.kind);
    let output_path = resolve_output_path(config)?;

    let mut operations = Vec::new();
    let updated_header = build_updated_header(&original_header.header, config, &mut operations)?;
    if operations.is_empty() {
        return Err(AppError::InvalidHeaderMutation {
            path: config.input_path.clone(),
            detail: "No header mutation was requested. Supply --header or one or more header mutation flags.".to_string(),
        });
    }

    let planning = plan_execution(
        config.requested_mode,
        original_header.header.raw_header_text.len(),
        serialize_sam_header_text(&updated_header).len(),
    );

    Ok(ReheaderPayload {
        format: "BAM",
        mutation: RequestedHeaderMutation { operations },
        planning,
        execution: ReheaderExecutionResult {
            mode_used: None,
            modified: false,
            dry_run: config.dry_run,
        },
        output: ReheaderOutputInfo {
            path: output_path.to_string_lossy().into_owned(),
            written: false,
            overwritten: None,
        },
        index: ReheaderIndexInfo {
            present_before,
            valid_after: if config.dry_run { None } else { Some(false) },
            reindex_requested: config.reindex,
            reindexed: false,
            kind: index_kind,
        },
        checksum_verification: ReheaderChecksumVerificationInfo {
            requested: config.verify_checksum,
            performed: false,
            mode: None,
            header_included: false,
            input_digest: None,
            output_digest: None,
            r#match: None,
        },
        notes: default_notes(),
    })
}

pub fn execute(config: &ReheaderConfig) -> Result<ReheaderExecution, AppError> {
    let original_header = parse_bam_header(&config.input_path)?;
    let mut ignored_operations = Vec::new();
    let updated_header =
        build_updated_header(&original_header.header, config, &mut ignored_operations)?;
    let output_path = resolve_output_path(config)?;
    let overwritten = output_path.exists();
    let mut payload = preview(config)?;
    validate_output_request(config, &output_path)?;

    if config.dry_run {
        payload
            .notes
            .push("Dry run only. No file modifications were made.".to_string());
        if config.reindex {
            payload.notes.push(
                "Reindexing was requested, but index writing is not implemented in this slice."
                    .to_string(),
            );
        }
        return Ok(ReheaderExecution { payload });
    }

    let mode_used = resolve_mode_used(config, &payload.planning)?;
    payload.execution.mode_used = Some(mode_used);
    payload.execution.modified = true;
    describe_mode_notes(&mut payload.notes, mode_used);

    let input_digest = if config.verify_checksum {
        Some(compute_alignment_only_checksum(&config.input_path)?)
    } else {
        None
    };

    match mode_used {
        ReheaderExecutionMode::RewriteMinimized | ReheaderExecutionMode::SafeRewrite => {
            rewrite_bam_header(
                &config.input_path,
                &output_path,
                &updated_header,
                config.force,
            )?
        }
        ReheaderExecutionMode::InPlace => {
            return Err(AppError::Unimplemented {
                path: config.input_path.clone(),
                detail: "True in-place BAM reheadering is not implemented in this slice."
                    .to_string(),
            });
        }
    }

    payload.output.written = true;
    payload.output.overwritten = Some(overwritten);

    if config.reindex {
        payload.notes.push(
            "Reindexing was requested, but index writing is not implemented in this slice. Any pre-existing index should be considered invalidated."
                .to_string(),
        );
        payload.index.kind = payload.index.kind.or(Some(IndexKind::Bai));
    } else if payload.index.present_before {
        payload.notes.push(
            "A pre-existing BAM index should be regenerated after reheader because header mutation invalidates index assumptions in this slice."
                .to_string(),
        );
    }

    if let Some(input_digest) = input_digest {
        let output_digest = compute_alignment_only_checksum(&output_path)?;
        let matched = input_digest == output_digest;
        payload.checksum_verification = ReheaderChecksumVerificationInfo {
            requested: true,
            performed: true,
            mode: Some(ChecksumMode::CanonicalRecordOrder),
            header_included: false,
            input_digest: Some(input_digest.clone()),
            output_digest: Some(output_digest.clone()),
            r#match: Some(matched),
        };

        if matched {
            payload.notes.push(
                "Canonical record-order checksum verification confirmed that alignment-record content was preserved while excluding header bytes from the checksum domain."
                    .to_string(),
            );
        } else {
            return Err(AppError::ChecksumMismatch {
                path: output_path,
                detail: format!(
                    "Input alignment-content checksum {input_digest} did not match output alignment-content checksum {output_digest}."
                ),
            });
        }
    }

    Ok(ReheaderExecution { payload })
}

fn build_updated_header(
    original: &BamHeaderView,
    config: &ReheaderConfig,
    operations: &mut Vec<ReheaderMutationOperation>,
) -> Result<BamHeaderView, AppError> {
    let mut header = if let Some(path) = &config.header_path {
        let header_text =
            fs::read_to_string(path).map_err(|error| AppError::from_io(path, error))?;
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::ReplaceHeader,
            source: Some(path.to_string_lossy().into_owned()),
            target_rg: None,
            details: None,
            comment: None,
        });
        parse_sam_header_text_with_references(&header_text, &original.references).map_err(
            |detail| AppError::InvalidHeaderFile {
                path: path.clone(),
                detail,
            },
        )?
    } else {
        original.clone()
    };

    for rg_spec in &config.add_rgs {
        let fields = parse_tag_assignment_list(rg_spec, "read-group", &config.input_path)?;
        let Some(id) = fields.get("ID").cloned() else {
            return Err(AppError::InvalidHeaderMutation {
                path: config.input_path.clone(),
                detail: "--add-rg requires an ID field.".to_string(),
            });
        };
        if header
            .read_groups
            .iter()
            .any(|existing| existing.id.as_deref() == Some(id.as_str()))
        {
            return Err(AppError::DuplicateReadGroup {
                path: config.input_path.clone(),
                id,
            });
        }
        let rg = read_group_from_fields(&fields);
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::AddRg,
            source: None,
            target_rg: rg.id.clone(),
            details: Some(fields),
            comment: None,
        });
        header.read_groups.push(rg);
    }

    for rg_spec in &config.set_rgs {
        let fields = parse_tag_assignment_list(rg_spec, "read-group", &config.input_path)?;
        let target_id =
            resolve_set_rg_target(&fields, config.target_rg.as_deref(), &config.input_path)?;
        let rg = find_read_group_mut(&mut header.read_groups, &target_id, &config.input_path)?;
        apply_rg_updates(rg, &fields);
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetRg,
            source: None,
            target_rg: Some(target_id),
            details: Some(fields),
            comment: None,
        });
    }

    for rg_id in &config.remove_rgs {
        let removed = remove_read_group(&mut header.read_groups, rg_id);
        if !removed {
            return Err(AppError::MissingReadGroup {
                path: config.input_path.clone(),
                id: rg_id.clone(),
            });
        }
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::RemoveRg,
            source: None,
            target_rg: Some(rg_id.clone()),
            details: None,
            comment: None,
        });
    }

    if let Some(sample) = &config.set_sample {
        let target_id = require_target_rg(config, "set sample")?;
        let rg = find_read_group_mut(&mut header.read_groups, &target_id, &config.input_path)?;
        rg.sample = Some(sample.clone());
        let mut details = BTreeMap::new();
        details.insert("SM".to_string(), sample.clone());
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetSample,
            source: None,
            target_rg: Some(target_id),
            details: Some(details),
            comment: None,
        });
    }

    if let Some(platform) = &config.set_platform {
        let target_id = require_target_rg(config, "set platform")?;
        let rg = find_read_group_mut(&mut header.read_groups, &target_id, &config.input_path)?;
        rg.platform = Some(platform.clone());
        let mut details = BTreeMap::new();
        details.insert("PL".to_string(), platform.clone());
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetPlatform,
            source: None,
            target_rg: Some(target_id),
            details: Some(details),
            comment: None,
        });
    }

    for pg_spec in &config.set_pgs {
        let fields = parse_tag_assignment_list(pg_spec, "program", &config.input_path)?;
        let Some(id) = fields.get("ID").cloned() else {
            return Err(AppError::InvalidHeaderMutation {
                path: config.input_path.clone(),
                detail: "--set-pg requires an ID field.".to_string(),
            });
        };
        if let Some(existing) = header
            .programs
            .iter_mut()
            .find(|program| program.id.as_deref() == Some(id.as_str()))
        {
            apply_pg_updates(existing, &fields);
        } else {
            header.programs.push(program_from_fields(&fields));
        }
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::SetPg,
            source: None,
            target_rg: None,
            details: Some(fields),
            comment: None,
        });
    }

    for comment in &config.add_comments {
        if comment.contains('\n') || comment.contains('\r') {
            return Err(AppError::InvalidHeaderMutation {
                path: config.input_path.clone(),
                detail: "Header comments may not contain embedded newlines.".to_string(),
            });
        }
        header.comments.push(comment.clone());
        operations.push(ReheaderMutationOperation {
            operation: ReheaderOperationKind::AddComment,
            source: None,
            target_rg: None,
            details: None,
            comment: Some(comment.clone()),
        });
    }

    header.raw_header_text = serialize_sam_header_text(&header);
    Ok(header)
}

fn parse_tag_assignment_list(
    raw: &str,
    context: &str,
    path: &Path,
) -> Result<BTreeMap<String, String>, AppError> {
    let mut fields = BTreeMap::new();
    for segment in raw.split(',') {
        let Some((key, value)) = segment.split_once('=') else {
            return Err(AppError::InvalidHeaderMutation {
                path: path.to_path_buf(),
                detail: format!(
                    "Malformed {context} field assignment: {segment}. Expected KEY=VALUE."
                ),
            });
        };

        if key.len() != 2
            || !key
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(AppError::InvalidHeaderMutation {
                path: path.to_path_buf(),
                detail: format!(
                    "Malformed {context} field tag {key}. Expected a two-character SAM header tag."
                ),
            });
        }
        if value.is_empty() {
            return Err(AppError::InvalidHeaderMutation {
                path: path.to_path_buf(),
                detail: format!(
                    "Malformed {context} field assignment {segment}. Empty values are not allowed."
                ),
            });
        }
        if value.contains('\t') || value.contains('\n') || value.contains('\r') {
            return Err(AppError::InvalidHeaderMutation {
                path: path.to_path_buf(),
                detail: format!(
                    "Malformed {context} field assignment {segment}. Embedded tabs or newlines are not allowed."
                ),
            });
        }

        fields.insert(key.to_string(), value.to_string());
    }

    if fields.is_empty() {
        return Err(AppError::InvalidHeaderMutation {
            path: path.to_path_buf(),
            detail: format!("No {context} fields were provided."),
        });
    }

    Ok(fields)
}

fn read_group_from_fields(fields: &BTreeMap<String, String>) -> ReadGroupRecord {
    let mut rg = ReadGroupRecord::default();
    apply_rg_updates(&mut rg, fields);
    rg
}

fn apply_rg_updates(rg: &mut ReadGroupRecord, fields: &BTreeMap<String, String>) {
    for (tag, value) in fields {
        match tag.as_str() {
            "ID" => rg.id = Some(value.clone()),
            "SM" => rg.sample = Some(value.clone()),
            "LB" => rg.library = Some(value.clone()),
            "PL" => rg.platform = Some(value.clone()),
            "PU" => rg.platform_unit = Some(value.clone()),
            "CN" => rg.center = Some(value.clone()),
            "DS" => rg.description = Some(value.clone()),
            "DT" => rg.date = Some(value.clone()),
            _ => {
                rg.other_fields.insert(tag.clone(), value.clone());
            }
        }
    }
}

fn program_from_fields(fields: &BTreeMap<String, String>) -> ProgramRecord {
    let mut program = ProgramRecord::default();
    apply_pg_updates(&mut program, fields);
    program
}

fn apply_pg_updates(program: &mut ProgramRecord, fields: &BTreeMap<String, String>) {
    for (tag, value) in fields {
        match tag.as_str() {
            "ID" => program.id = Some(value.clone()),
            "PN" => program.name = Some(value.clone()),
            "VN" => program.version = Some(value.clone()),
            "CL" => program.command_line = Some(value.clone()),
            "PP" => program.previous_program_id = Some(value.clone()),
            "DS" => program.description = Some(value.clone()),
            _ => {
                program.other_fields.insert(tag.clone(), value.clone());
            }
        }
    }
}

fn find_read_group_mut<'a>(
    read_groups: &'a mut [ReadGroupRecord],
    id: &str,
    path: &Path,
) -> Result<&'a mut ReadGroupRecord, AppError> {
    read_groups
        .iter_mut()
        .find(|rg| rg.id.as_deref() == Some(id))
        .ok_or_else(|| AppError::MissingReadGroup {
            path: path.to_path_buf(),
            id: id.to_string(),
        })
}

fn remove_read_group(read_groups: &mut Vec<ReadGroupRecord>, id: &str) -> bool {
    let original_len = read_groups.len();
    read_groups.retain(|rg| rg.id.as_deref() != Some(id));
    read_groups.len() != original_len
}

fn resolve_set_rg_target(
    fields: &BTreeMap<String, String>,
    target_rg: Option<&str>,
    path: &Path,
) -> Result<String, AppError> {
    match (fields.get("ID"), target_rg) {
        (Some(id), Some(target)) if id != target => Err(AppError::InvalidHeaderMutation {
            path: path.to_path_buf(),
            detail: format!(
                "--set-rg specified ID={id} but --target-rg requested {target}; these must match."
            ),
        }),
        (Some(id), _) => Ok(id.clone()),
        (None, Some(target)) => Ok(target.to_string()),
        (None, None) => Err(AppError::InvalidHeaderMutation {
            path: path.to_path_buf(),
            detail: "--set-rg requires an ID field or --target-rg.".to_string(),
        }),
    }
}

fn require_target_rg(config: &ReheaderConfig, action: &str) -> Result<String, AppError> {
    config
        .target_rg
        .clone()
        .ok_or_else(|| AppError::InvalidHeaderMutation {
            path: config.input_path.clone(),
            detail: format!("--target-rg is required to {action} in this slice."),
        })
}

fn plan_execution(
    mode_requested: ReheaderExecutionMode,
    original_header_len: usize,
    replacement_header_len: usize,
) -> ReheaderExecutionPlan {
    let in_place_feasible = false;
    let size_delta = replacement_header_len as i64 - original_header_len as i64;
    let reason = if replacement_header_len > original_header_len {
        format!(
            "Replacement header increases serialized size by {} bytes beyond safe in-place limits, and true BGZF header patching is not implemented safely in this slice.",
            size_delta
        )
    } else if replacement_header_len < original_header_len {
        format!(
            "Replacement header is {} bytes smaller, but true BGZF header patching is not implemented safely in this slice.",
            size_delta.unsigned_abs()
        )
    } else {
        "Replacement header is the same serialized size as the existing header, but true BGZF header patching is not implemented safely in this slice."
            .to_string()
    };

    let recommended_mode = match mode_requested {
        ReheaderExecutionMode::SafeRewrite => ReheaderExecutionMode::SafeRewrite,
        ReheaderExecutionMode::InPlace | ReheaderExecutionMode::RewriteMinimized => {
            ReheaderExecutionMode::RewriteMinimized
        }
    };

    ReheaderExecutionPlan {
        mode_requested,
        in_place_feasible,
        recommended_mode,
        reason: Some(reason),
    }
}

fn resolve_mode_used(
    config: &ReheaderConfig,
    planning: &ReheaderExecutionPlan,
) -> Result<ReheaderExecutionMode, AppError> {
    match config.requested_mode {
        ReheaderExecutionMode::InPlace if !planning.in_place_feasible => {
            if config.rewrite_fallback_permitted {
                Ok(planning.recommended_mode)
            } else {
                Err(AppError::InPlaceNotFeasible {
                    path: config.input_path.clone(),
                    detail: planning.reason.clone().unwrap_or_else(|| {
                        "True in-place execution could not be proven safe.".to_string()
                    }),
                })
            }
        }
        mode => Ok(mode),
    }
}

fn resolve_output_path(config: &ReheaderConfig) -> Result<PathBuf, AppError> {
    let path = if let Some(output_path) = &config.output_path {
        output_path.clone()
    } else {
        default_output_path(&config.input_path)
    };

    if path == config.input_path {
        return Err(AppError::UnsupportedReheaderMode {
            path: config.input_path.clone(),
            detail: "Same-path BAM rewrites are not implemented safely in this slice; provide --out for rewrite modes."
                .to_string(),
        });
    }

    Ok(path)
}

fn validate_output_request(config: &ReheaderConfig, output_path: &Path) -> Result<(), AppError> {
    if output_path.exists() && !config.force {
        return Err(AppError::OutputExists {
            path: output_path.to_path_buf(),
        });
    }

    Ok(())
}

fn default_output_path(input: &Path) -> PathBuf {
    let input_str = input.to_string_lossy();
    if let Some(stripped) = input_str.strip_suffix(".bam") {
        PathBuf::from(format!("{stripped}.reheadered.bam"))
    } else {
        PathBuf::from(format!("{input_str}.reheadered.bam"))
    }
}

fn rewrite_bam_header(
    input_path: &Path,
    output_path: &Path,
    updated_header: &BamHeaderView,
    force: bool,
) -> Result<(), AppError> {
    if output_path.exists() && !force {
        return Err(AppError::OutputExists {
            path: output_path.to_path_buf(),
        });
    }

    let temp_path = temporary_output_path(output_path);
    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(|error| AppError::WriteError {
            path: temp_path.clone(),
            message: error.to_string(),
        })?;
    }

    let mut reader = BamReader::open(input_path)?;
    let _header = parse_bam_header_from_reader(&mut reader)?;
    let mut writer = BgzfWriter::create(&temp_path)?;

    let header_payload =
        serialize_bam_header_payload(&updated_header.raw_header_text, &updated_header.references);
    writer.write_all(&header_payload)?;

    while let Some(record) = read_next_record_layout(&mut reader)? {
        let bytes = serialize_record_layout(&record);
        writer.write_all(&bytes)?;
    }

    writer.finish()?;

    if output_path.exists() {
        fs::remove_file(output_path).map_err(|error| AppError::WriteError {
            path: output_path.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::rename(&temp_path, output_path).map_err(|error| AppError::WriteError {
        path: output_path.to_path_buf(),
        message: error.to_string(),
    })?;

    Ok(())
}

fn temporary_output_path(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output.bam");
    output_path.with_file_name(format!(
        ".{file_name}.bamana-reheader-{}.tmp",
        std::process::id()
    ))
}

fn compute_alignment_only_checksum(path: &Path) -> Result<String, AppError> {
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

    extract_digest(
        compute_checksums(path, &options)?,
        ChecksumMode::CanonicalRecordOrder,
    )
    .ok_or_else(|| AppError::ChecksumUncertainty {
        path: path.to_path_buf(),
        detail: "Canonical checksum result was missing from the checksum response.".to_string(),
    })
}

fn default_notes() -> Vec<String> {
    vec![
        "Header-only mutation was applied or planned.".to_string(),
        "This command modifies only BAM header metadata; it does not add, remove, or replace per-record RG:Z tags in alignment records.".to_string(),
    ]
}

fn describe_mode_notes(notes: &mut Vec<String>, mode_used: ReheaderExecutionMode) {
    match mode_used {
        ReheaderExecutionMode::RewriteMinimized => notes.push(
            "The current rewrite-minimized path still rewrites the BAM container, but it preserves serialized alignment-record layout bytes directly rather than performing a deep semantic record transformation."
                .to_string(),
        ),
        ReheaderExecutionMode::SafeRewrite => notes.push(
            "The current safe-rewrite path rewrites the BAM container conservatively and does not attempt true in-place header patching."
                .to_string(),
        ),
        ReheaderExecutionMode::InPlace => notes.push(
            "True in-place mode remains reserved for future narrowly proven-safe cases."
                .to_string(),
        ),
    }
}
