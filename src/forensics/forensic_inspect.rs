use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    path::Path,
};

use serde::Serialize;

use crate::{
    bam::{
        header::{HeaderPayload, ProgramRecord, ReadGroupRecord, parse_bam_header_from_reader},
        reader::BamReader,
        records::{decode_bam_qualities, decode_bam_sequence, read_next_record_layout},
        tags::{extract_string_aux_tag, traverse_aux_fields},
    },
    error::AppError,
    forensics::duplication::{
        DuplicateRange, DuplicationFindingType, DuplicationIdentityMode, build_identity_key,
        detect_adjacent_duplicate_blocks,
    },
    formats::probe::DetectedFormat,
};

const DUPLICATION_IDENTITY_MODE: DuplicationIdentityMode = DuplicationIdentityMode::QnameSeqQual;
const DEFAULT_DUPLICATION_MIN_BLOCK_SIZE: usize = 10;
const REGIME_WINDOW_SIZE: usize = 256;
const WINDOW_DOMINANCE_FRACTION: f64 = 0.80;
const TAG_PRESENCE_FRACTION: f64 = 0.60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForensicScanMode {
    Bounded,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ForensicRecommendation {
    #[serde(rename = "inspect_duplication")]
    InspectDuplication,
    #[serde(rename = "deduplicate")]
    Deduplicate,
    #[serde(rename = "validate")]
    Validate,
    #[serde(rename = "checksum")]
    Checksum,
    #[serde(rename = "header")]
    Header,
    #[serde(rename = "reheader")]
    Reheader,
    #[serde(rename = "forensic_inspect --full-scan")]
    ForensicInspectFullScan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForensicFindingCategory {
    HeaderInconsistency,
    ReadGroupInconsistency,
    ProgramChainAnomaly,
    ReadNameRegimeShift,
    DuplicateBlockHallmark,
    ConcatenationHallmark,
    MixedRunSignature,
    ManualEditSuspicion,
    TagSchemaShift,
    OrderingAnomaly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ForensicSeverity {
    Info,
    Warning,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ForensicConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ForensicEvidenceStrength {
    Limited,
    Moderate,
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForensicEvidenceScope {
    HeaderOnly,
    BodyBounded,
    BodyFull,
    HeaderAndBodyBounded,
    HeaderAndBodyFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ForensicScope {
    pub header: bool,
    pub read_groups: bool,
    pub program_chain: bool,
    pub read_names: bool,
    pub tags: bool,
    pub duplication_hallmarks: bool,
}

#[derive(Debug, Clone)]
pub struct ForensicInspectConfig {
    pub record_limit: u64,
    pub max_findings: usize,
    pub scopes: ForensicScope,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForensicInspectPayload {
    pub format: DetectedFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_mode: Option<ForensicScanMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<ForensicScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_examined: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ForensicSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<ForensicFinding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment: Option<ForensicAssessment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForensicSummary {
    pub findings_total: usize,
    pub high_findings: usize,
    pub warning_findings: usize,
    pub info_findings: usize,
    pub category_counts: Vec<ForensicCategoryCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForensicCategoryCount {
    pub category: ForensicFindingCategory,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForensicFinding {
    pub category: ForensicFindingCategory,
    pub severity: ForensicSeverity,
    pub confidence: ForensicConfidence,
    pub evidence_strength: ForensicEvidenceStrength,
    pub evidence_scope: ForensicEvidenceScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_range_1: Option<DuplicateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_range_2: Option<DuplicateRange>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_follow_up: Option<Vec<ForensicRecommendation>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForensicAssessment {
    pub suspicion_detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub likely_concatenation_or_coercion: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_follow_up: Option<Vec<ForensicRecommendation>>,
}

#[derive(Debug)]
pub struct ForensicInspectionFailure {
    pub payload: ForensicInspectPayload,
    pub error: AppError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum ReadNameStyle {
    ColonDelimited,
    SpaceDelimited,
    SlashDelimited,
    UuidLike,
    StructuredToken,
    SimpleToken,
}

#[derive(Debug, Clone)]
struct WindowObservation {
    name_style: ReadNameStyle,
    tag_keys: Vec<String>,
}

#[derive(Debug, Clone)]
struct RegimeSegment {
    label: String,
    start_record: u64,
    end_record: u64,
}

#[derive(Debug)]
struct BodyScanState {
    records_examined: u64,
    reached_eof: bool,
    identities: Vec<usize>,
    identity_lookup: HashMap<String, usize>,
    first_window: Vec<WindowObservation>,
    last_window: VecDeque<WindowObservation>,
    rg_segments: Vec<RegimeSegment>,
    observed_unknown_rg_ids: BTreeSet<String>,
    records_with_rg: u64,
    missing_rg_records: u64,
}

pub fn inspect_path(
    path: &Path,
    format: DetectedFormat,
    config: &ForensicInspectConfig,
) -> Result<ForensicInspectPayload, ForensicInspectionFailure> {
    if format != DetectedFormat::Bam {
        return Err(ForensicInspectionFailure {
            payload: base_payload(format),
            error: AppError::UnsupportedInputForCommand {
                path: path.to_path_buf(),
                detail: format!(
                    "forensic_inspect currently supports BAM only in this first slice; detected {format}."
                ),
            },
        });
    }

    let (header, body_state) = load_bam(path, config).map_err(|(header, body_state, error)| {
        ForensicInspectionFailure {
            payload: partial_failure_payload(format, config, header.as_ref(), body_state.as_ref()),
            error,
        }
    })?;

    let reached_eof = body_state
        .as_ref()
        .map(|state| state.reached_eof)
        .unwrap_or(true);
    let scan_mode = if reached_eof {
        ForensicScanMode::Full
    } else {
        ForensicScanMode::Bounded
    };

    let mut findings = Vec::new();

    if config.scopes.header || config.scopes.read_groups {
        findings.extend(inspect_header_read_groups(
            &header,
            reached_eof,
            config.scopes,
        ));
    }
    if config.scopes.program_chain {
        findings.extend(inspect_program_chain(
            &header,
            needs_body_scan(config.scopes),
            reached_eof,
        ));
    }
    if let Some(body_state) = body_state.as_ref() {
        if config.scopes.read_groups {
            findings.extend(inspect_body_read_groups(&header, body_state, scan_mode));
        }
        if config.scopes.read_names {
            findings.extend(inspect_read_name_regime(body_state, scan_mode));
        }
        if config.scopes.tags {
            findings.extend(inspect_tag_schema(body_state, scan_mode));
        }
        if config.scopes.duplication_hallmarks {
            findings.extend(inspect_duplication_hallmarks(body_state, scan_mode));
        }
    }

    findings.sort_by_key(finding_sort_key);
    findings.truncate(config.max_findings.max(1));

    let summary = build_summary(&findings);
    let assessment = build_assessment(&findings, scan_mode, config.scopes);
    let notes = build_notes(config.scopes, reached_eof);

    Ok(ForensicInspectPayload {
        format,
        scan_mode: Some(scan_mode),
        scopes: Some(config.scopes),
        records_examined: Some(
            body_state
                .as_ref()
                .map_or(0, |state| state.records_examined),
        ),
        summary: Some(summary),
        findings: Some(findings),
        assessment: Some(assessment),
        notes: Some(notes),
    })
}

fn load_bam(
    path: &Path,
    config: &ForensicInspectConfig,
) -> Result<
    (HeaderPayload, Option<BodyScanState>),
    (Option<HeaderPayload>, Option<BodyScanState>, AppError),
> {
    let mut reader = BamReader::open(path).map_err(|error| (None, None, error))?;
    let header = parse_bam_header_from_reader(&mut reader).map_err(|error| {
        (
            None,
            None,
            AppError::ParseUncertainty {
                path: path.to_path_buf(),
                detail: error_detail(&error).unwrap_or_else(|| error.to_string()),
            },
        )
    })?;

    if !needs_body_scan(config.scopes) {
        return Ok((header, None));
    }

    let mut state = BodyScanState {
        records_examined: 0,
        reached_eof: false,
        identities: Vec::new(),
        identity_lookup: HashMap::new(),
        first_window: Vec::new(),
        last_window: VecDeque::with_capacity(REGIME_WINDOW_SIZE),
        rg_segments: Vec::new(),
        observed_unknown_rg_ids: BTreeSet::new(),
        records_with_rg: 0,
        missing_rg_records: 0,
    };
    let header_rg_ids = header
        .header
        .read_groups
        .iter()
        .filter_map(|rg| rg.id.clone())
        .collect::<HashSet<_>>();

    while state.records_examined < config.record_limit {
        let layout = match read_next_record_layout(&mut reader) {
            Ok(Some(layout)) => layout,
            Ok(None) => {
                state.reached_eof = true;
                break;
            }
            Err(
                AppError::InvalidRecord { detail, .. } | AppError::TruncatedFile { detail, .. },
            ) => {
                return Err((
                    Some(header),
                    Some(state),
                    AppError::ParseUncertainty {
                        path: path.to_path_buf(),
                        detail,
                    },
                ));
            }
            Err(error) => return Err((Some(header), Some(state), error)),
        };

        let read_group = if config.scopes.read_groups || config.scopes.tags {
            extract_string_aux_tag(&layout.aux_bytes, *b"RG").map_err(|detail| {
                (
                    Some(header.clone()),
                    Some(state_snapshot(&state)),
                    AppError::ParseUncertainty {
                        path: path.to_path_buf(),
                        detail,
                    },
                )
            })?
        } else {
            None
        };

        let tag_keys = if config.scopes.tags {
            collect_tag_keys(&layout.aux_bytes).map_err(|detail| {
                (
                    Some(header.clone()),
                    Some(state_snapshot(&state)),
                    AppError::ParseUncertainty {
                        path: path.to_path_buf(),
                        detail,
                    },
                )
            })?
        } else {
            Vec::new()
        };

        let name_style =
            if config.scopes.read_names || config.scopes.tags || config.scopes.read_groups {
                classify_read_name(&layout.read_name)
            } else {
                ReadNameStyle::SimpleToken
            };

        state.records_examined += 1;

        if config.scopes.duplication_hallmarks {
            let sequence =
                decode_bam_sequence(&layout.sequence_bytes, layout.l_seq).map_err(|detail| {
                    (
                        Some(header.clone()),
                        Some(state_snapshot(&state)),
                        AppError::ParseUncertainty {
                            path: path.to_path_buf(),
                            detail,
                        },
                    )
                })?;
            let quality = decode_bam_qualities(&layout.quality_bytes).map_err(|detail| {
                (
                    Some(header.clone()),
                    Some(state_snapshot(&state)),
                    AppError::ParseUncertainty {
                        path: path.to_path_buf(),
                        detail,
                    },
                )
            })?;
            let key = build_identity_key(
                DUPLICATION_IDENTITY_MODE,
                &layout.read_name,
                &sequence,
                Some(quality.as_str()),
                None,
            );
            let identity_id = intern_identity(&mut state.identity_lookup, &key);
            state.identities.push(identity_id);
        }

        if config.scopes.read_groups {
            let rg_label = read_group
                .clone()
                .unwrap_or_else(|| "<missing>".to_string());
            push_rg_segment(&mut state.rg_segments, &rg_label, state.records_examined);
            match read_group {
                Some(rg) => {
                    state.records_with_rg += 1;
                    if !header_rg_ids.contains(&rg) {
                        state.observed_unknown_rg_ids.insert(rg);
                    }
                }
                None => {
                    state.missing_rg_records += 1;
                }
            }
        }

        if config.scopes.read_names || config.scopes.tags || config.scopes.read_groups {
            let observation = WindowObservation {
                name_style,
                tag_keys,
            };
            if state.first_window.len() < REGIME_WINDOW_SIZE {
                state.first_window.push(observation.clone());
            }
            if state.last_window.len() == REGIME_WINDOW_SIZE {
                state.last_window.pop_front();
            }
            state.last_window.push_back(observation);
        }
    }

    Ok((header, Some(state)))
}

fn inspect_header_read_groups(
    header: &HeaderPayload,
    _reached_eof: bool,
    _scopes: ForensicScope,
) -> Vec<ForensicFinding> {
    let mut findings = Vec::new();
    let evidence_scope = ForensicEvidenceScope::HeaderOnly;

    let duplicate_rg_ids = duplicate_ids(
        header
            .header
            .read_groups
            .iter()
            .filter_map(|rg| rg.id.as_deref()),
    );
    if !duplicate_rg_ids.is_empty() {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::HeaderInconsistency,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::High,
            evidence_strength: ForensicEvidenceStrength::Strong,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "Header contains duplicate @RG identifiers: {}.",
                duplicate_rg_ids.join(", ")
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Header,
                ForensicRecommendation::Reheader,
            ]),
        });
    }

    let missing_rg_ids = header
        .header
        .read_groups
        .iter()
        .filter(|rg| rg.id.as_deref().is_none_or(str::is_empty))
        .count();
    if missing_rg_ids > 0 {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::HeaderInconsistency,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::High,
            evidence_strength: ForensicEvidenceStrength::Moderate,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "Header contains {missing_rg_ids} @RG record(s) without a stable ID field."
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Header,
                ForensicRecommendation::Reheader,
            ]),
        });
    }

    let sample_platforms = sample_platform_map(&header.header.read_groups);
    let mixed_platform_samples = sample_platforms
        .iter()
        .filter(|(_, platforms)| platforms.len() > 1)
        .map(|(sample, platforms)| format!("{sample} [{}]", platforms.join(", ")))
        .collect::<Vec<_>>();
    if !mixed_platform_samples.is_empty() {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::MixedRunSignature,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::Medium,
            evidence_strength: ForensicEvidenceStrength::Moderate,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "Header declares the same sample across multiple platform signatures: {}.",
                mixed_platform_samples.join("; ")
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Header,
                ForensicRecommendation::ForensicInspectFullScan,
            ]),
        });
    }

    if header.header.read_groups.is_empty()
        && header.header.programs.is_empty()
        && header.header.comments.is_empty()
    {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::ManualEditSuspicion,
            severity: ForensicSeverity::Info,
            confidence: ForensicConfidence::Low,
            evidence_strength: ForensicEvidenceStrength::Limited,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: "Header contains neither @RG nor @PG provenance records and no @CO comments; provenance context may be sparse or manually reduced.".to_string(),
            recommended_follow_up: Some(vec![ForensicRecommendation::Header]),
        });
    }

    findings
}

fn inspect_program_chain(
    header: &HeaderPayload,
    _body_scanned: bool,
    _reached_eof: bool,
) -> Vec<ForensicFinding> {
    let evidence_scope = ForensicEvidenceScope::HeaderOnly;
    let mut findings = Vec::new();
    let duplicate_pg_ids = duplicate_ids(
        header
            .header
            .programs
            .iter()
            .filter_map(|pg| pg.id.as_deref()),
    );
    if !duplicate_pg_ids.is_empty() {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::ProgramChainAnomaly,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::High,
            evidence_strength: ForensicEvidenceStrength::Strong,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "Header contains duplicate @PG identifiers: {}.",
                duplicate_pg_ids.join(", ")
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Header,
                ForensicRecommendation::Reheader,
            ]),
        });
    }

    let missing_pg_ids = header
        .header
        .programs
        .iter()
        .filter(|pg| pg.id.as_deref().is_none_or(str::is_empty))
        .count();
    if missing_pg_ids > 0 {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::ProgramChainAnomaly,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::High,
            evidence_strength: ForensicEvidenceStrength::Moderate,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "Header contains {missing_pg_ids} @PG record(s) without a stable ID field."
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Header,
                ForensicRecommendation::Reheader,
            ]),
        });
    }

    let program_ids = header
        .header
        .programs
        .iter()
        .filter_map(|pg| pg.id.clone())
        .collect::<HashSet<_>>();
    let broken_parents = header
        .header
        .programs
        .iter()
        .filter_map(|pg| {
            let parent = pg.previous_program_id.as_ref()?;
            (!program_ids.contains(parent)).then_some(parent.clone())
        })
        .collect::<BTreeSet<_>>();
    if !broken_parents.is_empty() {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::ProgramChainAnomaly,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::High,
            evidence_strength: ForensicEvidenceStrength::Moderate,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "Header contains @PG parent references that do not resolve locally: {}.",
                broken_parents.into_iter().collect::<Vec<_>>().join(", ")
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Header,
                ForensicRecommendation::Reheader,
            ]),
        });
    }

    let disconnected_roots = disconnected_program_roots(&header.header.programs);
    if disconnected_roots.len() > 1 {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::ProgramChainAnomaly,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::Medium,
            evidence_strength: ForensicEvidenceStrength::Moderate,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "Header contains multiple disconnected @PG chains: {}.",
                disconnected_roots.join(", ")
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Header,
                ForensicRecommendation::Reheader,
            ]),
        });
    }

    findings
}

fn inspect_body_read_groups(
    header: &HeaderPayload,
    body: &BodyScanState,
    scan_mode: ForensicScanMode,
) -> Vec<ForensicFinding> {
    let mut findings = Vec::new();
    let evidence_scope = body_scope(scan_mode);
    let header_has_rg = !header.header.read_groups.is_empty();

    if !body.observed_unknown_rg_ids.is_empty() {
        let mut follow_up = vec![
            ForensicRecommendation::Header,
            ForensicRecommendation::Validate,
        ];
        if header_has_rg {
            follow_up.push(ForensicRecommendation::Reheader);
        }
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::ReadGroupInconsistency,
            severity: ForensicSeverity::Warning,
            confidence: ForensicConfidence::High,
            evidence_strength: ForensicEvidenceStrength::Moderate,
            evidence_scope,
            record_range_1: None,
            record_range_2: None,
            message: if header_has_rg {
                format!(
                    "Observed RG tags in alignment records that are not declared in the BAM header: {}.",
                    body.observed_unknown_rg_ids.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            } else {
                "Observed RG tags in alignment records even though the BAM header declares no @RG records.".to_string()
            },
            recommended_follow_up: Some(follow_up),
        });
    }

    if header_has_rg && body.records_with_rg > 0 && body.missing_rg_records > 0 {
        let missing_fraction = body.missing_rg_records as f64 / body.records_examined.max(1) as f64;
        if missing_fraction >= 0.10 {
            findings.push(ForensicFinding {
                category: ForensicFindingCategory::ReadGroupInconsistency,
                severity: ForensicSeverity::Warning,
                confidence: if scan_mode == ForensicScanMode::Full {
                    ForensicConfidence::High
                } else {
                    ForensicConfidence::Medium
                },
                evidence_strength: if scan_mode == ForensicScanMode::Full {
                    ForensicEvidenceStrength::Moderate
                } else {
                    ForensicEvidenceStrength::Limited
                },
                evidence_scope,
                record_range_1: None,
                record_range_2: None,
                message: format!(
                    "{} of {} examined records lacked RG tags even though other records in the same BAM use them.",
                    body.missing_rg_records, body.records_examined
                ),
                recommended_follow_up: Some(vec![
                    ForensicRecommendation::Validate,
                    ForensicRecommendation::Header,
                    ForensicRecommendation::ForensicInspectFullScan,
                ]),
            });
        }
    }

    if let Some((first, second)) = dominant_rg_boundary(body) {
        findings.push(ForensicFinding {
            category: ForensicFindingCategory::OrderingAnomaly,
            severity: ForensicSeverity::Warning,
            confidence: if scan_mode == ForensicScanMode::Full {
                ForensicConfidence::High
            } else {
                ForensicConfidence::Medium
            },
            evidence_strength: if scan_mode == ForensicScanMode::Full {
                ForensicEvidenceStrength::Moderate
            } else {
                ForensicEvidenceStrength::Limited
            },
            evidence_scope,
            record_range_1: Some(DuplicateRange {
                start: first.start_record,
                end: first.end_record,
            }),
            record_range_2: Some(DuplicateRange {
                start: second.start_record,
                end: second.end_record,
            }),
            message: format!(
                "Observed a sharp RG regime transition from {} to {} across large contiguous record blocks, consistent with concatenation or coerced collection boundaries.",
                first.label, second.label
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::InspectDuplication,
                ForensicRecommendation::Header,
                ForensicRecommendation::ForensicInspectFullScan,
            ]),
        });
    }

    findings
}

fn inspect_read_name_regime(
    body: &BodyScanState,
    scan_mode: ForensicScanMode,
) -> Vec<ForensicFinding> {
    let Some(first_style) = dominant_name_style(&body.first_window) else {
        return Vec::new();
    };
    let last_window = body.last_window.iter().cloned().collect::<Vec<_>>();
    let Some(last_style) = dominant_name_style(&last_window) else {
        return Vec::new();
    };

    if first_style != last_style {
        return vec![ForensicFinding {
            category: ForensicFindingCategory::ReadNameRegimeShift,
            severity: if scan_mode == ForensicScanMode::Full {
                ForensicSeverity::High
            } else {
                ForensicSeverity::Warning
            },
            confidence: if scan_mode == ForensicScanMode::Full {
                ForensicConfidence::High
            } else {
                ForensicConfidence::Medium
            },
            evidence_strength: if scan_mode == ForensicScanMode::Full {
                ForensicEvidenceStrength::Strong
            } else {
                ForensicEvidenceStrength::Moderate
            },
            evidence_scope: body_scope(scan_mode),
            record_range_1: None,
            record_range_2: None,
            message: format!(
                "A sharp transition in read-name structure was observed between the early and late portions of the examined records ({} to {}).",
                read_name_style_label(first_style),
                read_name_style_label(last_style)
            ),
            recommended_follow_up: Some(vec![
                ForensicRecommendation::InspectDuplication,
                ForensicRecommendation::Checksum,
                ForensicRecommendation::ForensicInspectFullScan,
            ]),
        }];
    }

    Vec::new()
}

fn inspect_tag_schema(body: &BodyScanState, scan_mode: ForensicScanMode) -> Vec<ForensicFinding> {
    let first_summary = window_tag_summary(&body.first_window);
    let last_window = body.last_window.iter().cloned().collect::<Vec<_>>();
    let last_summary = window_tag_summary(&last_window);
    if first_summary.records < 16 || last_summary.records < 16 {
        return Vec::new();
    }

    let avg_tag_gap = (first_summary.avg_tags_per_record - last_summary.avg_tags_per_record).abs();
    let prevalent_differs = first_summary.prevalent_tags != last_summary.prevalent_tags
        && (!first_summary.prevalent_tags.is_empty() || !last_summary.prevalent_tags.is_empty());

    if avg_tag_gap >= 1.5 || prevalent_differs {
        return vec![ForensicFinding {
            category: ForensicFindingCategory::TagSchemaShift,
            severity: ForensicSeverity::Warning,
            confidence: if scan_mode == ForensicScanMode::Full {
                ForensicConfidence::Medium
            } else {
                ForensicConfidence::Medium
            },
            evidence_strength: if scan_mode == ForensicScanMode::Full {
                ForensicEvidenceStrength::Moderate
            } else {
                ForensicEvidenceStrength::Limited
            },
            evidence_scope: body_scope(scan_mode),
            record_range_1: None,
            record_range_2: None,
            message: if scan_mode == ForensicScanMode::Full {
                format!(
                    "Observed a shift in aux-tag usage across the examined BAM body (early prevalent tags: {}; late prevalent tags: {}).",
                    display_tag_set(&first_summary.prevalent_tags),
                    display_tag_set(&last_summary.prevalent_tags)
                )
            } else {
                "Observed a shift in aux-tag usage within the bounded scan; full-file significance is not established.".to_string()
            },
            recommended_follow_up: Some(vec![
                ForensicRecommendation::Validate,
                ForensicRecommendation::ForensicInspectFullScan,
            ]),
        }];
    }

    Vec::new()
}

fn inspect_duplication_hallmarks(
    body: &BodyScanState,
    scan_mode: ForensicScanMode,
) -> Vec<ForensicFinding> {
    detect_adjacent_duplicate_blocks(&body.identities, DEFAULT_DUPLICATION_MIN_BLOCK_SIZE)
        .into_iter()
        .map(|block| {
            let (category, message) = if block.finding_type == DuplicationFindingType::WholeFileAppendDuplicate {
                (
                    ForensicFindingCategory::ConcatenationHallmark,
                    "A large ordered block of read identities appears twice in direct succession, consistent with file append duplication.".to_string(),
                )
            } else {
                (
                    ForensicFindingCategory::DuplicateBlockHallmark,
                    format!(
                        "A contiguous block of {} read identities appears twice in immediate succession under qname_seq_qual identity.",
                        block.block_len
                    ),
                )
            };

            ForensicFinding {
                category,
                severity: ForensicSeverity::High,
                confidence: ForensicConfidence::High,
                evidence_strength: ForensicEvidenceStrength::Strong,
                evidence_scope: body_scope(scan_mode),
                record_range_1: Some(DuplicateRange {
                    start: block.first_start as u64 + 1,
                    end: (block.first_start + block.block_len) as u64,
                }),
                record_range_2: Some(DuplicateRange {
                    start: (block.first_start + block.block_len) as u64 + 1,
                    end: (block.first_start + block.block_len * 2) as u64,
                }),
                message,
                recommended_follow_up: Some(vec![
                    ForensicRecommendation::InspectDuplication,
                    ForensicRecommendation::Deduplicate,
                    ForensicRecommendation::Checksum,
                ]),
            }
        })
        .collect()
}

fn build_summary(findings: &[ForensicFinding]) -> ForensicSummary {
    let mut category_counts = BTreeMap::new();
    let mut high_findings = 0_usize;
    let mut warning_findings = 0_usize;
    let mut info_findings = 0_usize;

    for finding in findings {
        *category_counts.entry(finding.category).or_insert(0_usize) += 1;
        match finding.severity {
            ForensicSeverity::High => high_findings += 1,
            ForensicSeverity::Warning => warning_findings += 1,
            ForensicSeverity::Info => info_findings += 1,
        }
    }

    ForensicSummary {
        findings_total: findings.len(),
        high_findings,
        warning_findings,
        info_findings,
        category_counts: category_counts
            .into_iter()
            .map(|(category, count)| ForensicCategoryCount { category, count })
            .collect(),
    }
}

fn build_assessment(
    findings: &[ForensicFinding],
    scan_mode: ForensicScanMode,
    scopes: ForensicScope,
) -> ForensicAssessment {
    let suspicion_detected = !findings.is_empty();
    let likely_concatenation_or_coercion = if findings.iter().any(is_strong_concatenation_signal) {
        Some(true)
    } else if !suspicion_detected {
        if scan_mode == ForensicScanMode::Full && needs_body_scan(scopes) {
            Some(false)
        } else if !needs_body_scan(scopes) {
            None
        } else {
            None
        }
    } else if scan_mode == ForensicScanMode::Full {
        Some(false)
    } else {
        None
    };

    let mut recommended_follow_up = BTreeSet::new();
    if suspicion_detected {
        for finding in findings {
            if let Some(recommendations) = &finding.recommended_follow_up {
                recommended_follow_up.extend(recommendations.iter().copied());
            }
        }
        if findings
            .iter()
            .any(|finding| finding.category == ForensicFindingCategory::ProgramChainAnomaly)
        {
            recommended_follow_up.insert(ForensicRecommendation::Header);
        }
    } else if scan_mode == ForensicScanMode::Bounded && needs_body_scan(scopes) {
        recommended_follow_up.insert(ForensicRecommendation::ForensicInspectFullScan);
    }

    ForensicAssessment {
        suspicion_detected,
        likely_concatenation_or_coercion,
        recommended_follow_up: (!recommended_follow_up.is_empty())
            .then_some(recommended_follow_up.into_iter().collect()),
    }
}

fn build_notes(scopes: ForensicScope, reached_eof: bool) -> Vec<String> {
    let mut notes = vec![
        "forensic_inspect reports provenance anomalies and coercion hallmarks; it is not a structural validator.".to_string(),
        "forensic_inspect is not duplicate marking and does not make biological claims about PCR or molecular duplication.".to_string(),
        "Forensic findings indicate provenance anomalies or collection mishandling, not biological interpretation or fraud attribution.".to_string(),
        "Duplication hallmarks in this slice use qname_seq_qual identity for BAM body inspection.".to_string(),
    ];

    if scopes.tags {
        notes.push(
            "Aux-tag inspection in this slice is selective and regime-oriented; it does not attempt exhaustive semantic interpretation of every auxiliary field.".to_string(),
        );
    }

    if needs_body_scan(scopes) && !reached_eof {
        notes.push(
            "The body scan stopped at the bounded record limit before EOF, so body-oriented findings are limited to bounded evidence and absence of findings is not a whole-file proof.".to_string(),
        );
    }

    notes
}

fn partial_failure_payload(
    format: DetectedFormat,
    config: &ForensicInspectConfig,
    header: Option<&HeaderPayload>,
    body_state: Option<&BodyScanState>,
) -> ForensicInspectPayload {
    let findings = header
        .map(|header| {
            let mut findings = Vec::new();
            if config.scopes.header || config.scopes.read_groups {
                findings.extend(inspect_header_read_groups(header, false, config.scopes));
            }
            if config.scopes.program_chain {
                findings.extend(inspect_program_chain(
                    header,
                    needs_body_scan(config.scopes),
                    false,
                ));
            }
            findings
        })
        .unwrap_or_default();
    let summary = (!findings.is_empty()).then(|| build_summary(&findings));

    ForensicInspectPayload {
        format,
        scan_mode: None,
        scopes: Some(config.scopes),
        records_examined: body_state.map(|state| state.records_examined),
        summary,
        findings: (!findings.is_empty()).then_some(findings),
        assessment: None,
        notes: Some(vec![
            "Forensic inspection did not complete cleanly enough to support a stable provenance assessment.".to_string(),
        ]),
    }
}

fn base_payload(format: DetectedFormat) -> ForensicInspectPayload {
    ForensicInspectPayload {
        format,
        scan_mode: None,
        scopes: None,
        records_examined: None,
        summary: None,
        findings: None,
        assessment: None,
        notes: None,
    }
}

fn collect_tag_keys(aux_bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut tags = Vec::new();
    traverse_aux_fields(aux_bytes, |field| {
        tags.push(String::from_utf8_lossy(&field.tag).into_owned());
        Ok(())
    })?;
    tags.sort();
    Ok(tags)
}

fn duplicate_ids<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0_usize) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(value, count)| (count > 1).then_some(value))
        .collect()
}

fn sample_platform_map(read_groups: &[ReadGroupRecord]) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::<String, BTreeSet<String>>::new();
    for rg in read_groups {
        let Some(sample) = rg.sample.as_ref() else {
            continue;
        };
        let Some(platform) = rg.platform.as_ref() else {
            continue;
        };
        map.entry(sample.clone())
            .or_default()
            .insert(platform.clone());
    }

    map.into_iter()
        .map(|(sample, platforms)| (sample, platforms.into_iter().collect()))
        .collect()
}

fn disconnected_program_roots(programs: &[ProgramRecord]) -> Vec<String> {
    let ids = programs
        .iter()
        .filter_map(|program| program.id.clone())
        .collect::<HashSet<_>>();
    let mut roots = BTreeSet::new();

    for program in programs {
        let Some(id) = program.id.as_ref() else {
            continue;
        };
        let has_parent = program
            .previous_program_id
            .as_ref()
            .is_some_and(|parent| ids.contains(parent));
        if !has_parent {
            roots.insert(id.clone());
        }
    }

    roots.into_iter().collect()
}

fn classify_read_name(read_name: &str) -> ReadNameStyle {
    let colon_count = read_name.matches(':').count();
    if colon_count >= 3 {
        return ReadNameStyle::ColonDelimited;
    }
    if read_name.contains(' ') {
        return ReadNameStyle::SpaceDelimited;
    }
    if read_name.contains('/') {
        return ReadNameStyle::SlashDelimited;
    }
    if looks_uuid_like(read_name) {
        return ReadNameStyle::UuidLike;
    }
    if read_name.contains('_') || read_name.contains('-') {
        return ReadNameStyle::StructuredToken;
    }
    ReadNameStyle::SimpleToken
}

fn looks_uuid_like(value: &str) -> bool {
    let segments = value.split('-').collect::<Vec<_>>();
    if segments.len() != 5 {
        return false;
    }
    segments.iter().all(|segment| {
        !segment.is_empty()
            && segment.len() <= 12
            && segment
                .chars()
                .all(|character| character.is_ascii_hexdigit())
    })
}

fn dominant_name_style(window: &[WindowObservation]) -> Option<ReadNameStyle> {
    if window.len() < 16 {
        return None;
    }
    let mut counts = BTreeMap::new();
    for observation in window {
        *counts.entry(observation.name_style).or_insert(0_usize) += 1;
    }
    let (&style, &count) = counts.iter().max_by_key(|(_, count)| **count)?;
    ((count as f64 / window.len() as f64) >= WINDOW_DOMINANCE_FRACTION).then_some(style)
}

fn read_name_style_label(style: ReadNameStyle) -> &'static str {
    match style {
        ReadNameStyle::ColonDelimited => "colon-delimited",
        ReadNameStyle::SpaceDelimited => "space-delimited",
        ReadNameStyle::SlashDelimited => "slash-delimited",
        ReadNameStyle::UuidLike => "uuid-like",
        ReadNameStyle::StructuredToken => "structured-token",
        ReadNameStyle::SimpleToken => "simple-token",
    }
}

#[derive(Debug)]
struct WindowTagSummary {
    records: usize,
    avg_tags_per_record: f64,
    prevalent_tags: BTreeSet<String>,
}

fn window_tag_summary(window: &[WindowObservation]) -> WindowTagSummary {
    let records = window.len();
    if records == 0 {
        return WindowTagSummary {
            records: 0,
            avg_tags_per_record: 0.0,
            prevalent_tags: BTreeSet::new(),
        };
    }

    let mut tag_counts = BTreeMap::<String, usize>::new();
    let mut total_tags = 0_usize;
    for observation in window {
        total_tags += observation.tag_keys.len();
        let unique_tags = observation
            .tag_keys
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        for tag in unique_tags {
            *tag_counts.entry(tag).or_insert(0_usize) += 1;
        }
    }

    WindowTagSummary {
        records,
        avg_tags_per_record: total_tags as f64 / records as f64,
        prevalent_tags: tag_counts
            .into_iter()
            .filter_map(|(tag, count)| {
                ((count as f64 / records as f64) >= TAG_PRESENCE_FRACTION).then_some(tag)
            })
            .collect(),
    }
}

fn display_tag_set(tags: &BTreeSet<String>) -> String {
    if tags.is_empty() {
        "none".to_string()
    } else {
        tags.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

fn push_rg_segment(segments: &mut Vec<RegimeSegment>, label: &str, record_number: u64) {
    if let Some(last) = segments.last_mut() {
        if last.label == label {
            last.end_record = record_number;
            return;
        }
    }

    segments.push(RegimeSegment {
        label: label.to_string(),
        start_record: record_number,
        end_record: record_number,
    });
}

fn dominant_rg_boundary(body: &BodyScanState) -> Option<(RegimeSegment, RegimeSegment)> {
    if body.rg_segments.len() < 2 || body.records_examined < 100 {
        return None;
    }
    let threshold = (body.records_examined / 10).max(50);
    let large_segments = body
        .rg_segments
        .iter()
        .filter(|segment| {
            segment.label != "<missing>"
                && segment.end_record - segment.start_record + 1 >= threshold
        })
        .cloned()
        .collect::<Vec<_>>();
    if large_segments.len() < 2 {
        return None;
    }

    let first = &large_segments[0];
    let second = &large_segments[1];
    (first.label != second.label).then_some((first.clone(), second.clone()))
}

fn body_scope(scan_mode: ForensicScanMode) -> ForensicEvidenceScope {
    match scan_mode {
        ForensicScanMode::Bounded => ForensicEvidenceScope::BodyBounded,
        ForensicScanMode::Full => ForensicEvidenceScope::BodyFull,
    }
}

fn needs_body_scan(scopes: ForensicScope) -> bool {
    scopes.read_groups || scopes.read_names || scopes.tags || scopes.duplication_hallmarks
}

fn finding_sort_key(
    finding: &ForensicFinding,
) -> (
    std::cmp::Reverse<u8>,
    std::cmp::Reverse<u8>,
    std::cmp::Reverse<u8>,
    ForensicFindingCategory,
    String,
) {
    (
        std::cmp::Reverse(severity_rank(finding.severity)),
        std::cmp::Reverse(confidence_rank(finding.confidence)),
        std::cmp::Reverse(evidence_rank(finding.evidence_strength)),
        finding.category,
        finding.message.clone(),
    )
}

fn severity_rank(severity: ForensicSeverity) -> u8 {
    match severity {
        ForensicSeverity::Info => 0,
        ForensicSeverity::Warning => 1,
        ForensicSeverity::High => 2,
    }
}

fn confidence_rank(confidence: ForensicConfidence) -> u8 {
    match confidence {
        ForensicConfidence::Low => 0,
        ForensicConfidence::Medium => 1,
        ForensicConfidence::High => 2,
    }
}

fn evidence_rank(strength: ForensicEvidenceStrength) -> u8 {
    match strength {
        ForensicEvidenceStrength::Limited => 0,
        ForensicEvidenceStrength::Moderate => 1,
        ForensicEvidenceStrength::Strong => 2,
    }
}

fn is_strong_concatenation_signal(finding: &ForensicFinding) -> bool {
    matches!(
        finding.category,
        ForensicFindingCategory::ConcatenationHallmark
            | ForensicFindingCategory::DuplicateBlockHallmark
            | ForensicFindingCategory::ReadNameRegimeShift
            | ForensicFindingCategory::OrderingAnomaly
    ) && finding.severity == ForensicSeverity::High
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

fn error_detail(error: &AppError) -> Option<String> {
    match error {
        AppError::InvalidHeader { detail, .. }
        | AppError::InvalidRecord { detail, .. }
        | AppError::ParseUncertainty { detail, .. }
        | AppError::InvalidBam { detail, .. }
        | AppError::TruncatedFile { detail, .. } => Some(detail.clone()),
        _ => None,
    }
}

fn state_snapshot(state: &BodyScanState) -> BodyScanState {
    BodyScanState {
        records_examined: state.records_examined,
        reached_eof: state.reached_eof,
        identities: state.identities.clone(),
        identity_lookup: state.identity_lookup.clone(),
        first_window: state.first_window.clone(),
        last_window: state.last_window.clone(),
        rg_segments: state.rg_segments.clone(),
        observed_unknown_rg_ids: state.observed_unknown_rg_ids.clone(),
        records_with_rg: state.records_with_rg,
        missing_rg_records: state.missing_rg_records,
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
        formats::probe::DetectedFormat,
    };

    use super::{
        ForensicFindingCategory, ForensicInspectConfig, ForensicRecommendation, ForensicScope,
        inspect_path,
    };

    #[test]
    fn detects_concatenation_hallmark_in_bam() {
        let path =
            std::env::temp_dir().join(format!("bamana-forensic-dup-{}.bam", std::process::id()));
        let mut records = Vec::new();
        for index in 0..10 {
            records.push(build_test_record(
                &format!("read:{index}:A"),
                "ACGT",
                "!!!!",
                Some("rg1"),
                &[("NM", b'i', &[1, 0, 0, 0])],
            ));
        }
        let repeated = records.clone();
        records.extend(repeated);
        write_test_bam(
            &path,
            "@HD\tVN:1.6\tSO:unknown\n@RG\tID:rg1\tSM:s1\tPL:ILLUMINA\n@PG\tID:pg1\tPN:bamana\n",
            records,
        );

        let payload = inspect_path(&path, DetectedFormat::Bam, &full_scan_config(&path))
            .expect("inspection should succeed");
        fs::remove_file(path).expect("fixture should be removable");

        let findings = payload.findings.expect("findings should be present");
        assert!(findings.iter().any(|finding| {
            finding.category == ForensicFindingCategory::ConcatenationHallmark
                && finding
                    .recommended_follow_up
                    .as_ref()
                    .is_some_and(|follow_up| {
                        follow_up.contains(&ForensicRecommendation::Deduplicate)
                    })
        }));
        assert!(
            payload
                .assessment
                .expect("assessment should be present")
                .likely_concatenation_or_coercion
                .expect("full scan should support a stable boolean")
        );
    }

    #[test]
    fn detects_read_group_mismatch_between_header_and_body() {
        let path =
            std::env::temp_dir().join(format!("bamana-forensic-rg-{}.bam", std::process::id()));
        write_test_bam(
            &path,
            "@HD\tVN:1.6\tSO:unknown\n@RG\tID:rg1\tSM:s1\tPL:ILLUMINA\n@PG\tID:pg1\tPN:bamana\n",
            vec![
                build_test_record("read1", "ACGT", "!!!!", Some("rg_missing"), &[]),
                build_test_record("read2", "TGCA", "####", Some("rg_missing"), &[]),
            ],
        );

        let payload = inspect_path(&path, DetectedFormat::Bam, &full_scan_config(&path))
            .expect("inspection should succeed");
        fs::remove_file(path).expect("fixture should be removable");

        let findings = payload.findings.expect("findings should be present");
        assert!(findings.iter().any(|finding| {
            finding.category == ForensicFindingCategory::ReadGroupInconsistency
                && finding.message.contains("not declared in the BAM header")
        }));
    }

    #[test]
    fn detects_program_chain_anomaly_from_duplicate_pg_ids() {
        let path =
            std::env::temp_dir().join(format!("bamana-forensic-pg-{}.bam", std::process::id()));
        write_test_bam(
            &path,
            "@HD\tVN:1.6\tSO:unknown\n@RG\tID:rg1\tSM:s1\tPL:ILLUMINA\n@PG\tID:pg1\tPN:aligner\n@PG\tID:pg1\tPN:other\n",
            vec![build_test_record("read1", "ACGT", "!!!!", Some("rg1"), &[])],
        );

        let payload = inspect_path(&path, DetectedFormat::Bam, &full_scan_config(&path))
            .expect("inspection should succeed");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(
            payload
                .findings
                .expect("findings should be present")
                .iter()
                .any(|finding| finding.category == ForensicFindingCategory::ProgramChainAnomaly)
        );
    }

    #[test]
    fn reports_clean_bam_without_findings() {
        let path =
            std::env::temp_dir().join(format!("bamana-forensic-clean-{}.bam", std::process::id()));
        let records = (0..20)
            .map(|index| {
                build_test_record(
                    &format!("read:{index}:A"),
                    "ACGT",
                    "!!!!",
                    Some("rg1"),
                    &[("NM", b'i', &[1, 0, 0, 0])],
                )
            })
            .collect::<Vec<_>>();
        write_test_bam(
            &path,
            "@HD\tVN:1.6\tSO:unknown\n@RG\tID:rg1\tSM:s1\tPL:ILLUMINA\n@PG\tID:pg1\tPN:bamana\n",
            records,
        );

        let payload = inspect_path(&path, DetectedFormat::Bam, &full_scan_config(&path))
            .expect("inspection should succeed");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(payload.findings.as_ref().map(Vec::len), Some(0));
        assert_eq!(
            payload
                .assessment
                .expect("assessment should be present")
                .likely_concatenation_or_coercion,
            Some(false)
        );
    }

    fn full_scan_config(path: &PathBuf) -> ForensicInspectConfig {
        ForensicInspectConfig {
            record_limit: u64::MAX,
            max_findings: 25,
            scopes: ForensicScope {
                header: true,
                read_groups: true,
                program_chain: true,
                read_names: true,
                tags: true,
                duplication_hallmarks: true,
            },
        }
    }

    fn build_test_record(
        read_name: &str,
        sequence: &str,
        quality: &str,
        read_group: Option<&str>,
        extra_tags: &[(&str, u8, &[u8])],
    ) -> RecordLayout {
        let mut aux_bytes = Vec::new();
        if let Some(read_group) = read_group {
            aux_bytes.extend_from_slice(b"RG");
            aux_bytes.push(b'Z');
            aux_bytes.extend_from_slice(read_group.as_bytes());
            aux_bytes.push(0);
        }
        for (tag, type_code, payload) in extra_tags {
            aux_bytes.extend_from_slice(tag.as_bytes());
            aux_bytes.push(*type_code);
            aux_bytes.extend_from_slice(payload);
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

    fn write_test_bam(path: &PathBuf, header_text: &str, records: Vec<RecordLayout>) {
        let header_payload = serialize_bam_header_payload(
            header_text,
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
