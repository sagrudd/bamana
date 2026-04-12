use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    bam::{
        checksum::{
            ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions, compute_checksums,
            extract_digest,
        },
        header::{
            BamHeaderView, ReadGroupRecord, parse_bam_header, parse_bam_header_from_reader,
            serialize_bam_header_payload, serialize_sam_header_text,
        },
        index::{IndexKind, discover_index_candidates},
        reader::BamReader,
        records::{RecordLayout, read_next_record_layout},
        tags::traverse_aux_fields,
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "kebab-case")]
pub enum AnnotateRgMode {
    OnlyMissing,
    ReplaceExisting,
    FailOnConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotateRgHeaderPolicy {
    RequireExisting,
    CreateIfMissing,
    AddHeaderRg,
    SetHeaderRg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotateRgExecutionMode {
    SafeRewrite,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgRequestInfo {
    pub rg_id: String,
    pub record_mode: AnnotateRgMode,
    pub header_policy: AnnotateRgHeaderPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgExecutionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_used: Option<AnnotateRgExecutionMode>,
    pub modified: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgRecordSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_examined: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_annotated: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_already_matching: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_missing_rg_before: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_conflicting_before: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgHeaderInfo {
    pub rg_present_before: bool,
    pub rg_present_after: bool,
    pub header_modified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgOutputInfo {
    pub path: String,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwritten: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgIndexInfo {
    pub present_before: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_after: Option<bool>,
    pub reindex_requested: bool,
    pub reindexed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<IndexKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgChecksumVerificationInfo {
    pub requested: bool,
    pub performed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ChecksumMode>,
    pub excluded_tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotateRgPayload {
    pub format: &'static str,
    pub request: AnnotateRgRequestInfo,
    pub execution: AnnotateRgExecutionInfo,
    pub records: AnnotateRgRecordSummary,
    pub header: AnnotateRgHeaderInfo,
    pub output: AnnotateRgOutputInfo,
    pub index: AnnotateRgIndexInfo,
    pub checksum_verification: AnnotateRgChecksumVerificationInfo,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AnnotateRgConfig {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub rg_id: String,
    pub record_mode: AnnotateRgMode,
    pub header_policy: AnnotateRgHeaderPolicy,
    pub add_header_rg: Option<String>,
    pub set_header_rg: Option<String>,
    pub dry_run: bool,
    pub force: bool,
    pub reindex: bool,
    pub verify_checksum: bool,
    pub threads: usize,
}

#[derive(Debug, Clone)]
pub struct AnnotateRgExecution {
    pub payload: AnnotateRgPayload,
}

#[derive(Debug, Clone)]
struct AnnotationPlan {
    payload: AnnotateRgPayload,
    updated_header: BamHeaderView,
}

#[derive(Debug)]
struct RecordMutationOutcome {
    record: RecordLayout,
    annotated: bool,
    already_matching: bool,
    missing_before: bool,
    conflicting_before: bool,
}

pub fn preview(config: &AnnotateRgConfig) -> Result<AnnotateRgPayload, AppError> {
    Ok(build_plan(config)?.payload)
}

pub fn execute(config: &AnnotateRgConfig) -> Result<AnnotateRgExecution, AppError> {
    let plan = build_plan(config)?;
    let output_path = resolve_output_path(config);
    validate_output_request(config, &output_path)?;
    let overwritten = output_path.exists();

    let mut payload = plan.payload;
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
        return Ok(AnnotateRgExecution { payload });
    }

    let input_digest = if config.verify_checksum {
        Some(compute_rg_excluded_checksum(&config.input_path)?)
    } else {
        None
    };

    let summary = rewrite_with_rg_annotation(config, &output_path, &plan.updated_header)?;

    payload.execution.mode_used = Some(AnnotateRgExecutionMode::SafeRewrite);
    payload.execution.modified = true;
    payload.records.records_examined = Some(summary.records_examined);
    payload.records.records_annotated = Some(summary.records_annotated);
    payload.records.records_already_matching = Some(summary.records_already_matching);
    payload.records.records_missing_rg_before = Some(summary.records_missing_rg_before);
    payload.records.records_conflicting_before = Some(summary.records_conflicting_before);
    payload.output.written = true;
    payload.output.overwritten = Some(overwritten);

    if config.threads > 1 {
        payload.notes.push(
            "Requested thread count was accepted for contract stability, but this slice still executes annotate_rg as a single-stream rewrite."
                .to_string(),
        );
    }

    if config.reindex {
        payload.notes.push(
            "Reindexing was requested, but index writing is not implemented in this slice. Any pre-existing index should be considered invalidated."
                .to_string(),
        );
        payload.index.kind = payload.index.kind.or(Some(IndexKind::Bai));
    } else if payload.index.present_before {
        payload.notes.push(
            "A pre-existing BAM index should be regenerated after annotate_rg because record-level mutation invalidates index assumptions in this slice."
                .to_string(),
        );
    }

    if let Some(input_digest) = input_digest {
        let output_digest = compute_rg_excluded_checksum(&output_path)?;
        let matched = input_digest == output_digest;
        payload.checksum_verification = AnnotateRgChecksumVerificationInfo {
            requested: true,
            performed: true,
            mode: Some(ChecksumMode::CanonicalRecordOrder),
            excluded_tags: vec!["RG".to_string()],
            input_digest: Some(input_digest.clone()),
            output_digest: Some(output_digest.clone()),
            r#match: Some(matched),
        };
        if matched {
            payload.notes.push(
                "Canonical record-order checksum verification excluding RG confirmed that only read-group annotation changed within the checksum domain."
                    .to_string(),
            );
        } else {
            return Err(AppError::ChecksumMismatch {
                path: output_path,
                detail: format!(
                    "Input RG-excluded checksum {input_digest} did not match output RG-excluded checksum {output_digest}."
                ),
            });
        }
    }

    Ok(AnnotateRgExecution { payload })
}

fn build_plan(config: &AnnotateRgConfig) -> Result<AnnotationPlan, AppError> {
    let header_payload = parse_bam_header(&config.input_path)?;
    let index_candidates = discover_index_candidates(&config.input_path, false);
    let present_before = !index_candidates.is_empty();
    let index_kind = index_candidates.first().map(|candidate| candidate.kind);
    let output_path = resolve_output_path(config);

    let (updated_header, rg_present_before, rg_present_after, header_modified) =
        apply_header_policy(&header_payload.header, config)?;

    Ok(AnnotationPlan {
        payload: AnnotateRgPayload {
            format: "BAM",
            request: AnnotateRgRequestInfo {
                rg_id: config.rg_id.clone(),
                record_mode: config.record_mode,
                header_policy: config.header_policy,
            },
            execution: AnnotateRgExecutionInfo {
                mode_used: None,
                modified: false,
                dry_run: config.dry_run,
            },
            records: AnnotateRgRecordSummary {
                records_examined: None,
                records_annotated: None,
                records_already_matching: None,
                records_missing_rg_before: None,
                records_conflicting_before: None,
            },
            header: AnnotateRgHeaderInfo {
                rg_present_before,
                rg_present_after,
                header_modified,
            },
            output: AnnotateRgOutputInfo {
                path: output_path.to_string_lossy().into_owned(),
                written: false,
                overwritten: None,
            },
            index: AnnotateRgIndexInfo {
                present_before,
                valid_after: if config.dry_run { None } else { Some(false) },
                reindex_requested: config.reindex,
                reindexed: false,
                kind: index_kind,
            },
            checksum_verification: AnnotateRgChecksumVerificationInfo {
                requested: config.verify_checksum,
                performed: false,
                mode: None,
                excluded_tags: if config.verify_checksum {
                    vec!["RG".to_string()]
                } else {
                    Vec::new()
                },
                input_digest: None,
                output_digest: None,
                r#match: None,
            },
            notes: default_notes(),
        },
        updated_header,
    })
}

fn apply_header_policy(
    original: &BamHeaderView,
    config: &AnnotateRgConfig,
) -> Result<(BamHeaderView, bool, bool, bool), AppError> {
    let mut header = original.clone();
    let rg_present_before = header
        .read_groups
        .iter()
        .any(|rg| rg.id.as_deref() == Some(config.rg_id.as_str()));

    match config.header_policy {
        AnnotateRgHeaderPolicy::RequireExisting => {
            if !rg_present_before {
                return Err(AppError::MissingReadGroup {
                    path: config.input_path.clone(),
                    id: config.rg_id.clone(),
                });
            }
        }
        AnnotateRgHeaderPolicy::CreateIfMissing => {
            if !rg_present_before {
                header.read_groups.push(ReadGroupRecord {
                    id: Some(config.rg_id.clone()),
                    ..ReadGroupRecord::default()
                });
            }
        }
        AnnotateRgHeaderPolicy::AddHeaderRg => {
            if rg_present_before {
                return Err(AppError::DuplicateReadGroup {
                    path: config.input_path.clone(),
                    id: config.rg_id.clone(),
                });
            }
            let spec =
                config
                    .add_header_rg
                    .as_deref()
                    .ok_or_else(|| AppError::InvalidRgRequest {
                        path: config.input_path.clone(),
                        detail: "--add-header-rg requires a field specification.".to_string(),
                    })?;
            let fields = parse_header_field_assignments(spec, &config.input_path)?;
            header
                .read_groups
                .push(read_group_from_fields(normalize_rg_fields(
                    fields,
                    &config.rg_id,
                    &config.input_path,
                )?));
        }
        AnnotateRgHeaderPolicy::SetHeaderRg => {
            let spec =
                config
                    .set_header_rg
                    .as_deref()
                    .ok_or_else(|| AppError::InvalidRgRequest {
                        path: config.input_path.clone(),
                        detail: "--set-header-rg requires a field specification.".to_string(),
                    })?;
            let fields = parse_header_field_assignments(spec, &config.input_path)?;
            let normalized = normalize_rg_fields(fields, &config.rg_id, &config.input_path)?;
            let rg = header
                .read_groups
                .iter_mut()
                .find(|rg| rg.id.as_deref() == Some(config.rg_id.as_str()))
                .ok_or_else(|| AppError::MissingReadGroup {
                    path: config.input_path.clone(),
                    id: config.rg_id.clone(),
                })?;
            apply_rg_updates(rg, &normalized);
        }
    }

    let rg_present_after = header
        .read_groups
        .iter()
        .any(|rg| rg.id.as_deref() == Some(config.rg_id.as_str()));
    header.raw_header_text = serialize_sam_header_text(&header);
    let header_modified = header.raw_header_text != original.raw_header_text;
    Ok((header, rg_present_before, rg_present_after, header_modified))
}

fn parse_header_field_assignments(
    raw: &str,
    path: &Path,
) -> Result<BTreeMap<String, String>, AppError> {
    let mut fields = BTreeMap::new();
    for segment in raw.split(',') {
        let Some((key, value)) = segment.split_once('=') else {
            return Err(AppError::InvalidRgRequest {
                path: path.to_path_buf(),
                detail: format!(
                    "Malformed header RG field assignment {segment}. Expected KEY=VALUE."
                ),
            });
        };
        if key.len() != 2
            || !key
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(AppError::InvalidRgRequest {
                path: path.to_path_buf(),
                detail: format!(
                    "Malformed header RG field tag {key}. Expected a two-character SAM header tag."
                ),
            });
        }
        if value.is_empty() || value.contains('\t') || value.contains('\n') || value.contains('\r')
        {
            return Err(AppError::InvalidRgRequest {
                path: path.to_path_buf(),
                detail: format!("Malformed header RG field assignment {segment}."),
            });
        }
        fields.insert(key.to_string(), value.to_string());
    }
    if fields.is_empty() {
        return Err(AppError::InvalidRgRequest {
            path: path.to_path_buf(),
            detail: "No header RG fields were provided.".to_string(),
        });
    }
    Ok(fields)
}

fn normalize_rg_fields(
    mut fields: BTreeMap<String, String>,
    rg_id: &str,
    path: &Path,
) -> Result<BTreeMap<String, String>, AppError> {
    if let Some(id) = fields.get("ID") {
        if id != rg_id {
            return Err(AppError::InvalidRgRequest {
                path: path.to_path_buf(),
                detail: format!(
                    "Header RG field specification used ID={id}, but --rg-id requested {rg_id}."
                ),
            });
        }
    } else {
        fields.insert("ID".to_string(), rg_id.to_string());
    }
    Ok(fields)
}

fn read_group_from_fields(fields: BTreeMap<String, String>) -> ReadGroupRecord {
    let mut rg = ReadGroupRecord::default();
    apply_rg_updates(&mut rg, &fields);
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

fn rewrite_with_rg_annotation(
    config: &AnnotateRgConfig,
    output_path: &Path,
    updated_header: &BamHeaderView,
) -> Result<RecordRewriteSummary, AppError> {
    let temp_path = temporary_output_path(output_path);
    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(|error| AppError::WriteError {
            path: temp_path.clone(),
            message: error.to_string(),
        })?;
    }

    let mut reader = BamReader::open(&config.input_path)?;
    let _ = parse_bam_header_from_reader(&mut reader)?;
    let mut writer = BgzfWriter::create(&temp_path)?;
    let header_payload =
        serialize_bam_header_payload(&updated_header.raw_header_text, &updated_header.references);
    writer.write_all(&header_payload)?;

    let mut summary = RecordRewriteSummary::default();
    while let Some(record) = read_next_record_layout(&mut reader)? {
        let outcome = annotate_record_rg(
            record,
            &config.rg_id,
            config.record_mode,
            &config.input_path,
        )?;
        summary.records_examined += 1;
        summary.records_annotated += outcome.annotated as u64;
        summary.records_already_matching += outcome.already_matching as u64;
        summary.records_missing_rg_before += outcome.missing_before as u64;
        summary.records_conflicting_before += outcome.conflicting_before as u64;
        writer.write_all(&serialize_record_layout(&outcome.record))?;
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

    Ok(summary)
}

fn annotate_record_rg(
    mut record: RecordLayout,
    requested_id: &str,
    mode: AnnotateRgMode,
    path: &Path,
) -> Result<RecordMutationOutcome, AppError> {
    let mut preserved_aux = Vec::with_capacity(record.aux_bytes.len() + requested_id.len() + 4);
    let mut observed_values = Vec::new();

    traverse_aux_fields(&record.aux_bytes, |field| {
        if field.tag == *b"RG" {
            if field.type_code != b'Z' {
                return Err(format!(
                    "Record {} contained RG aux data with type {} instead of Z.",
                    record.read_name, field.type_code as char
                ));
            }
            observed_values.push(parse_aux_z_string(field.payload).map_err(|detail| {
                format!(
                    "Record {} contained malformed RG aux data: {detail}",
                    record.read_name
                )
            })?);
        } else {
            preserved_aux.extend_from_slice(&field.tag);
            preserved_aux.push(field.type_code);
            preserved_aux.extend_from_slice(field.payload);
        }
        Ok(())
    })
    .map_err(|detail| AppError::InvalidBam {
        path: path.to_path_buf(),
        detail,
    })?;

    if observed_values.len() > 1 {
        return Err(AppError::ConflictingReadGroupTags {
            path: path.to_path_buf(),
            detail: format!(
                "Observed duplicate RG tags on record {} while annotating read group {}.",
                record.read_name, requested_id
            ),
        });
    }

    let Some(existing_rg) = observed_values.first() else {
        append_rg_tag(&mut preserved_aux, requested_id);
        record.aux_bytes = preserved_aux;
        return Ok(RecordMutationOutcome {
            record,
            annotated: true,
            already_matching: false,
            missing_before: true,
            conflicting_before: false,
        });
    };

    if existing_rg == requested_id {
        return Ok(RecordMutationOutcome {
            record,
            annotated: false,
            already_matching: true,
            missing_before: false,
            conflicting_before: false,
        });
    }

    match mode {
        AnnotateRgMode::OnlyMissing => Ok(RecordMutationOutcome {
            record,
            annotated: false,
            already_matching: false,
            missing_before: false,
            conflicting_before: true,
        }),
        AnnotateRgMode::ReplaceExisting => {
            append_rg_tag(&mut preserved_aux, requested_id);
            record.aux_bytes = preserved_aux;
            Ok(RecordMutationOutcome {
                record,
                annotated: true,
                already_matching: false,
                missing_before: false,
                conflicting_before: true,
            })
        }
        AnnotateRgMode::FailOnConflict => Err(AppError::ConflictingReadGroupTags {
            path: path.to_path_buf(),
            detail: format!(
                "Observed record {} with RG value {} different from requested {}.",
                record.read_name, existing_rg, requested_id
            ),
        }),
    }
}

fn parse_aux_z_string(payload: &[u8]) -> Result<String, String> {
    let Some((&0, body)) = payload.split_last() else {
        return Err("unterminated Z-style aux string".to_string());
    };
    String::from_utf8(body.to_vec())
        .map_err(|error| format!("aux string was not valid UTF-8: {error}"))
}

fn append_rg_tag(aux_bytes: &mut Vec<u8>, rg_id: &str) {
    aux_bytes.extend_from_slice(b"RG");
    aux_bytes.push(b'Z');
    aux_bytes.extend_from_slice(rg_id.as_bytes());
    aux_bytes.push(0);
}

fn validate_output_request(config: &AnnotateRgConfig, output_path: &Path) -> Result<(), AppError> {
    if output_path.exists() && !config.force {
        return Err(AppError::OutputExists {
            path: output_path.to_path_buf(),
        });
    }
    Ok(())
}

fn resolve_output_path(config: &AnnotateRgConfig) -> PathBuf {
    config
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(&config.input_path))
}

fn default_output_path(input: &Path) -> PathBuf {
    let input_str = input.to_string_lossy();
    if let Some(stripped) = input_str.strip_suffix(".bam") {
        PathBuf::from(format!("{stripped}.annotated_rg.bam"))
    } else {
        PathBuf::from(format!("{input_str}.annotated_rg.bam"))
    }
}

fn temporary_output_path(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output.bam");
    output_path.with_file_name(format!(
        ".{file_name}.bamana-annotate-rg-{}.tmp",
        std::process::id()
    ))
}

fn compute_rg_excluded_checksum(path: &Path) -> Result<String, AppError> {
    let mut excluded_tags = HashSet::new();
    excluded_tags.insert(*b"RG");
    let options = ChecksumOptions {
        mode: ChecksumMode::CanonicalRecordOrder,
        algorithm: ChecksumAlgorithm::Sha256,
        include_header: false,
        excluded_tags,
        excluded_tag_strings: vec!["RG".to_string()],
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
        "Per-record RG tags were modified or planned.".to_string(),
        "This command is distinct from header-only reheader and may rewrite every BAM alignment record.".to_string(),
    ]
}

#[derive(Debug, Default)]
struct RecordRewriteSummary {
    records_examined: u64,
    records_annotated: u64,
    records_already_matching: u64,
    records_missing_rg_before: u64,
    records_conflicting_before: u64,
}
