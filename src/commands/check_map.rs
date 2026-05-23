use std::{collections::HashMap, path::PathBuf};

use crate::{
    bam::{
        header::HeaderPayload,
        index::{
            BaiIndexSummary, IndexKind, IndexResolution, bam_newer_than_index, parse_bai,
            parse_bai_index, resolve_index_for_bam,
        },
        record::BamRecordView,
        region::{NormalizedRegion, normalize_region_strings},
        region_plan::plan_bai_region_chunks,
        region_traversal::{RegionMatchedRecord, traverse_planned_region_chunks},
        scan::BamScanner,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
};
use serde::Serialize;

#[derive(Debug)]
pub struct CheckMapRequest {
    pub bam: PathBuf,
    pub sample_records: usize,
    pub full_scan: bool,
    pub prefer_index: bool,
    pub regions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckMapPayload {
    pub format: &'static str,
    pub mapping_status: MappingStatus,
    pub has_mapped_reads: Option<bool>,
    pub evidence_source: EvidenceSource,
    pub index: IndexInfo,
    pub references: Vec<ReferenceMappingInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_scope: Option<RegionScope>,
    pub summary: MappingSummary,
    pub confidence: ConfidenceLevel,
    pub semantic_note: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MappingStatus {
    Mapped,
    Unmapped,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceSource {
    Index,
    Scan,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionExecution {
    Indexed,
    ScanFallback,
    Rejected,
}

#[derive(Debug, Serialize)]
pub struct RegionScope {
    pub requested: bool,
    pub source: &'static str,
    pub coordinate_base: &'static str,
    pub interval_semantics: &'static str,
    pub duplicate_policy: &'static str,
    pub regions: Vec<NormalizedRegion>,
    pub execution: RegionExecution,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_records_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks_traversed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_records_seen: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_records_suppressed: Option<usize>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IndexInfo {
    pub present: bool,
    pub kind: Option<IndexKind>,
    pub used: bool,
}

#[derive(Debug, Serialize)]
pub struct ReferenceMappingInfo {
    pub name: String,
    pub length: u32,
    pub mapped_reads: Option<u64>,
    pub unmapped_reads: Option<u64>,
    pub observed: Option<bool>,
}

#[derive(Debug, Default, Serialize)]
pub struct MappingSummary {
    pub references_defined: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references_with_mapped_reads: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_mapped_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_unmapped_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_examined: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapped_records_observed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmapped_records_observed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references_with_mapped_reads_observed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inconsistent_records_observed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_records_examined: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_mapped_records_observed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_unmapped_records_observed: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Default)]
struct ScanState {
    records_examined: usize,
    mapped_records_observed: u64,
    unmapped_records_observed: u64,
    inconsistent_records_observed: u64,
    mapped_per_reference: HashMap<usize, u64>,
    placed_unmapped_per_reference: HashMap<usize, u64>,
}

struct RegionScopeOptions {
    regions: crate::bam::region::NormalizedRegionSet,
    execution: RegionExecution,
    index_path: Option<String>,
    fallback_mode: Option<&'static str>,
    scan_records_limit: Option<usize>,
    chunks_traversed: Option<usize>,
    raw_records_seen: Option<usize>,
    duplicate_records_suppressed: Option<usize>,
    note: String,
}

pub fn run(request: CheckMapRequest) -> Result<CheckMapPayload, AppError> {
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

    let mut scanner = BamScanner::open(&request.bam)?;
    let references_defined = scanner.header().header.references.len();

    if !request.regions.is_empty() {
        let regions = normalize_region_strings(
            &request.regions,
            &scanner.header().header.references,
            &request.bam,
        )?;
        return run_region_check_map(request, scanner, references_defined, regions);
    }

    let (index_info, index_note, index_summary) = if request.prefer_index {
        attempt_index_summary(&request.bam, references_defined)?
    } else {
        (
            IndexInfo {
                present: false,
                kind: None,
                used: false,
            },
            Some("Index preference was disabled; mapping assessment used scan mode.".to_string()),
            None,
        )
    };

    if let Some(index_summary) = index_summary {
        return Ok(build_index_payload(
            scanner.header(),
            index_info,
            index_summary,
            index_note,
        ));
    }

    let (scan_state, reached_eof) = scan_mapping_records(
        &mut scanner,
        request.sample_records.max(1),
        request.full_scan,
    )?;

    if scan_state.inconsistent_records_observed > 0
        && scan_state.mapped_records_observed == 0
        && !reached_eof
        && !request.full_scan
    {
        return Err(AppError::ParseUncertainty {
            path: request.bam,
            detail: "Alignment stream contained contradictory mapping fields before a confident conclusion was reached.".to_string(),
        });
    }

    Ok(build_scan_payload(
        scanner.header(),
        index_info,
        index_note,
        scan_state,
        reached_eof,
        request.full_scan,
    ))
}

fn run_region_check_map(
    request: CheckMapRequest,
    mut scanner: BamScanner,
    references_defined: usize,
    regions: crate::bam::region::NormalizedRegionSet,
) -> Result<CheckMapPayload, AppError> {
    if request.prefer_index {
        match attempt_region_index_payload(
            &request,
            scanner.header(),
            references_defined,
            &regions,
        )? {
            Some(payload) => return Ok(payload),
            None => {}
        }
    }

    let (scan_state, reached_eof) = scan_region_mapping_records(
        &mut scanner,
        &regions.regions,
        request.sample_records.max(1),
        request.full_scan,
        &request.bam,
    )?;
    let (index_info, fallback_reason) = region_fallback_index_info(&request, references_defined)?;

    let note = if reached_eof || request.full_scan {
        "Region-scoped mapping evidence is derived from native scan fallback and cannot be interpreted as whole-file mapping evidence."
    } else {
        "Region-scoped mapping evidence is derived from bounded native scan fallback and cannot be interpreted as whole-file mapping evidence."
    };
    Ok(build_region_payload(
        scanner.header(),
        index_info,
        scan_state,
        RegionScopeOptions {
            regions,
            execution: RegionExecution::ScanFallback,
            index_path: None,
            fallback_mode: Some("native_scan_required"),
            scan_records_limit: Some(if request.full_scan {
                usize::MAX
            } else {
                request.sample_records.max(1)
            }),
            chunks_traversed: None,
            raw_records_seen: None,
            duplicate_records_suppressed: None,
            note: match fallback_reason {
                Some(reason) => format!("{note} {reason}"),
                None => note.to_string(),
            },
        },
        ConfidenceLevel::Medium,
    ))
}

fn region_fallback_index_info(
    request: &CheckMapRequest,
    references_defined: usize,
) -> Result<(IndexInfo, Option<String>), AppError> {
    if !request.prefer_index {
        return Ok((
            IndexInfo {
                present: false,
                kind: None,
                used: false,
            },
            Some("Index preference was disabled for region-scoped check_map.".to_string()),
        ));
    }

    match resolve_index_for_bam(&request.bam) {
        IndexResolution::Present(resolved) => {
            let index_info = IndexInfo {
                present: true,
                kind: Some(resolved.kind),
                used: false,
            };
            if bam_newer_than_index(&request.bam, &resolved.path) == Some(true) {
                return Ok((
                    index_info,
                    Some(
                        "BAI index was present but timestamp-stale for region traversal."
                            .to_string(),
                    ),
                ));
            }
            match parse_bai_index(&resolved.path, references_defined) {
                Ok(_) => Ok((
                    index_info,
                    Some(
                        "BAI index was present but could not supply indexed region evidence; falling back to native scan."
                            .to_string(),
                    ),
                )),
                Err(AppError::UnsupportedIndex { detail, .. })
                | Err(AppError::InvalidIndex { detail, .. }) => Ok((
                    index_info,
                    Some(format!(
                        "BAI index was present but unusable for region traversal: {detail}"
                    )),
                )),
                Err(error) => Err(error),
            }
        }
        IndexResolution::Unsupported(resolved) => Ok((
            IndexInfo {
                present: true,
                kind: Some(resolved.kind),
                used: false,
            },
            Some(format!(
                "{} index detected, but it is not usable for region traversal.",
                index_kind_label(resolved.kind)
            )),
        )),
        IndexResolution::NotFound => Ok((
            IndexInfo {
                present: false,
                kind: None,
                used: false,
            },
            Some("No usable BAM index was found for region traversal.".to_string()),
        )),
    }
}

fn attempt_region_index_payload(
    request: &CheckMapRequest,
    header: &HeaderPayload,
    references_defined: usize,
    regions: &crate::bam::region::NormalizedRegionSet,
) -> Result<Option<CheckMapPayload>, AppError> {
    let resolved = match resolve_index_for_bam(&request.bam) {
        IndexResolution::Present(resolved) => resolved,
        IndexResolution::Unsupported(_) | IndexResolution::NotFound => return Ok(None),
    };
    if bam_newer_than_index(&request.bam, &resolved.path) == Some(true) {
        return Ok(None);
    }

    let index = match parse_bai_index(&resolved.path, references_defined) {
        Ok(index) => index,
        Err(AppError::UnsupportedIndex { .. }) | Err(AppError::InvalidIndex { .. }) => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let plan = match plan_bai_region_chunks(regions, &index, &resolved.path, resolved.kind, false) {
        Ok(plan) => plan,
        Err(AppError::UnsupportedIndex { .. })
        | Err(AppError::MissingIndex { .. })
        | Err(AppError::InvalidIndex { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let traversal = traverse_planned_region_chunks(&request.bam, regions, &plan)?;
    let scan_state = scan_state_from_region_records(&traversal.records, &request.bam)?;

    Ok(Some(build_region_payload(
        header,
        IndexInfo {
            present: true,
            kind: Some(resolved.kind),
            used: true,
        },
        scan_state,
        RegionScopeOptions {
            regions: regions.clone(),
            execution: RegionExecution::Indexed,
            index_path: Some(resolved.path.to_string_lossy().to_string()),
            fallback_mode: None,
            scan_records_limit: None,
            chunks_traversed: Some(traversal.chunks_traversed),
            raw_records_seen: Some(traversal.raw_records_seen),
            duplicate_records_suppressed: Some(traversal.duplicate_records_suppressed),
            note: "Region-scoped mapping evidence is derived from validated indexed traversal and cannot be interpreted as whole-file mapping evidence.".to_string(),
        },
        ConfidenceLevel::High,
    )))
}

fn attempt_index_summary(
    bam_path: &std::path::Path,
    references_defined: usize,
) -> Result<(IndexInfo, Option<String>, Option<BaiIndexSummary>), AppError> {
    match resolve_index_for_bam(bam_path) {
        IndexResolution::Present(resolved) => {
            if bam_newer_than_index(bam_path, &resolved.path) == Some(true) {
                return Ok((
                    IndexInfo {
                        present: true,
                        kind: Some(resolved.kind),
                        used: false,
                    },
                    Some(
                        "BAI index was present but timestamp-stale; falling back to alignment scan."
                            .to_string(),
                    ),
                    None,
                ));
            }

            match parse_bai(&resolved.path, references_defined) {
                Ok(summary) if summary.reference_summaries.iter().all(Option::is_some) => Ok((
                    IndexInfo {
                        present: true,
                        kind: Some(resolved.kind),
                        used: true,
                    },
                    Some("Discovered BAI sidecar passed structural validation and supplied complete per-reference mapped/unmapped metadata.".to_string()),
                    Some(summary),
                )),
                Ok(_) => Ok((
                    IndexInfo {
                        present: true,
                        kind: Some(resolved.kind),
                        used: false,
                    },
                    Some(
                        "BAI index was present, but per-reference mapped/unmapped metadata was incomplete; falling back to alignment scan."
                            .to_string(),
                    ),
                    None,
                )),
                Err(AppError::UnsupportedIndex { detail, .. }) => Ok((
                    IndexInfo {
                        present: true,
                        kind: Some(resolved.kind),
                        used: false,
                    },
                    Some(format!("{detail} Falling back to alignment scan.")),
                    None,
                )),
                Err(AppError::InvalidIndex { detail, .. }) => Ok((
                    IndexInfo {
                        present: true,
                        kind: Some(resolved.kind),
                        used: false,
                    },
                    Some(format!(
                        "BAI index was present but unusable: {detail} Falling back to alignment scan."
                    )),
                    None,
                )),
                Err(error) => Err(error),
            }
        }
        IndexResolution::Unsupported(resolved) => Ok((
            IndexInfo {
                present: true,
                kind: Some(resolved.kind),
                used: false,
            },
            Some(format!(
                "{} index detected, but it is not usable for check_map index-derived evidence in this slice; falling back to alignment scan.",
                index_kind_label(resolved.kind)
            )),
            None,
        )),
        IndexResolution::NotFound => Ok((
            IndexInfo {
                present: false,
                kind: None,
                used: false,
            },
            None,
            None,
        )),
    }
}

fn index_kind_label(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Bai => "BAI",
        IndexKind::Csi => "CSI",
        IndexKind::Gzi => "GZI",
        IndexKind::Unknown => "UNKNOWN",
    }
}

fn build_region_payload(
    header: &HeaderPayload,
    index_info: IndexInfo,
    scan_state: ScanState,
    options: RegionScopeOptions,
    confidence: ConfidenceLevel,
) -> CheckMapPayload {
    let references_with_mapped_reads_observed = scan_state.mapped_per_reference.len();
    let references = header
        .header
        .references
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            let mapped_reads = scan_state
                .mapped_per_reference
                .get(&index)
                .copied()
                .unwrap_or(0);
            let unmapped_reads = scan_state
                .placed_unmapped_per_reference
                .get(&index)
                .copied()
                .unwrap_or(0);

            ReferenceMappingInfo {
                name: reference.name.clone(),
                length: reference.length,
                mapped_reads: Some(mapped_reads),
                unmapped_reads: Some(unmapped_reads),
                observed: Some(mapped_reads > 0),
            }
        })
        .collect::<Vec<_>>();

    let has_mapped_reads = scan_state.mapped_records_observed > 0;
    let mapping_status = if has_mapped_reads {
        MappingStatus::Mapped
    } else if scan_state.records_examined > 0 {
        MappingStatus::Unmapped
    } else {
        MappingStatus::Indeterminate
    };

    CheckMapPayload {
        format: "BAM",
        mapping_status,
        has_mapped_reads: Some(has_mapped_reads),
        evidence_source: match options.execution {
            RegionExecution::Indexed => EvidenceSource::Index,
            RegionExecution::ScanFallback | RegionExecution::Rejected => EvidenceSource::Scan,
        },
        index: index_info,
        references,
        region_scope: Some(RegionScope {
            requested: true,
            source: "cli_regions",
            coordinate_base: options.regions.coordinate_base,
            interval_semantics: options.regions.interval_semantics,
            duplicate_policy: options.regions.duplicate_policy,
            regions: options.regions.regions,
            execution: options.execution,
            index_path: options.index_path,
            fallback_mode: options.fallback_mode,
            scan_records_limit: options.scan_records_limit,
            chunks_traversed: options.chunks_traversed,
            raw_records_seen: options.raw_records_seen,
            duplicate_records_suppressed: options.duplicate_records_suppressed,
            notes: vec![options.note.clone()],
        }),
        summary: MappingSummary {
            references_defined: header.header.references.len(),
            references_with_mapped_reads: None,
            total_mapped_reads: None,
            total_unmapped_reads: None,
            records_examined: None,
            mapped_records_observed: None,
            unmapped_records_observed: None,
            references_with_mapped_reads_observed: Some(references_with_mapped_reads_observed),
            inconsistent_records_observed: Some(scan_state.inconsistent_records_observed),
            region_records_examined: Some(scan_state.records_examined),
            region_mapped_records_observed: Some(scan_state.mapped_records_observed),
            region_unmapped_records_observed: Some(scan_state.unmapped_records_observed),
        },
        confidence,
        semantic_note: options.note,
    }
}

fn build_index_payload(
    header: &HeaderPayload,
    index_info: IndexInfo,
    index_summary: BaiIndexSummary,
    index_note: Option<String>,
) -> CheckMapPayload {
    let mut references_with_mapped_reads = 0_usize;
    let mut total_mapped_reads = 0_u64;
    let mut total_unmapped_reads = index_summary.unplaced_unmapped_reads.unwrap_or(0);

    let references = header
        .header
        .references
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            let counts = index_summary.reference_summaries[index]
                .as_ref()
                .expect("complete index counts were required before use");
            if counts.mapped_reads > 0 {
                references_with_mapped_reads += 1;
            }
            total_mapped_reads += counts.mapped_reads;
            total_unmapped_reads += counts.unmapped_reads;

            ReferenceMappingInfo {
                name: reference.name.clone(),
                length: reference.length,
                mapped_reads: Some(counts.mapped_reads),
                unmapped_reads: Some(counts.unmapped_reads),
                observed: None,
            }
        })
        .collect::<Vec<_>>();

    let has_mapped_reads = total_mapped_reads > 0;
    let mapping_status = if has_mapped_reads {
        MappingStatus::Mapped
    } else {
        MappingStatus::Unmapped
    };

    let semantic_note = match index_note {
        Some(note) => format!(
            "Mapping summary is derived from the BAM index and header; full validation of all alignment records was not performed. {note}"
        ),
        None => "Mapping summary is derived from the BAM index and header; full validation of all alignment records was not performed.".to_string(),
    };

    CheckMapPayload {
        format: "BAM",
        mapping_status,
        has_mapped_reads: Some(has_mapped_reads),
        evidence_source: EvidenceSource::Index,
        index: index_info,
        references,
        region_scope: None,
        summary: MappingSummary {
            references_defined: header.header.references.len(),
            references_with_mapped_reads: Some(references_with_mapped_reads),
            total_mapped_reads: Some(total_mapped_reads),
            total_unmapped_reads: Some(total_unmapped_reads),
            records_examined: None,
            mapped_records_observed: None,
            unmapped_records_observed: None,
            references_with_mapped_reads_observed: None,
            inconsistent_records_observed: None,
            region_records_examined: None,
            region_mapped_records_observed: None,
            region_unmapped_records_observed: None,
        },
        confidence: ConfidenceLevel::High,
        semantic_note,
    }
}

fn scan_mapping_records(
    scanner: &mut BamScanner,
    sample_records: usize,
    full_scan: bool,
) -> Result<(ScanState, bool), AppError> {
    let mut state = ScanState::default();
    let mut reached_eof = false;

    loop {
        if !full_scan && state.records_examined >= sample_records {
            break;
        }

        match scanner.next_record()? {
            Some(record_view) => {
                state.records_examined += 1;
                update_scan_state(&mut state, &record_view);
            }
            None => {
                reached_eof = true;
                break;
            }
        }
    }

    Ok((state, reached_eof))
}

fn scan_region_mapping_records(
    scanner: &mut BamScanner,
    regions: &[NormalizedRegion],
    sample_records: usize,
    full_scan: bool,
    path: &std::path::Path,
) -> Result<(ScanState, bool), AppError> {
    let mut state = ScanState::default();
    let mut reached_eof = false;
    let mut scanned_records = 0;

    loop {
        if !full_scan && scanned_records >= sample_records {
            break;
        }

        match scanner.next_record()? {
            Some(record_view) => {
                scanned_records += 1;
                if record_overlaps_regions(&record_view, regions, path)? {
                    state.records_examined += 1;
                    update_scan_state(&mut state, &record_view);
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

fn scan_state_from_region_records(
    records: &[RegionMatchedRecord],
    path: &std::path::Path,
) -> Result<ScanState, AppError> {
    let mut state = ScanState::default();
    for record in records {
        let view =
            BamRecordView::parse(&record.raw_record).map_err(|error| AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: error.detail().to_string(),
            })?;
        state.records_examined += 1;
        update_scan_state(&mut state, &view);
    }
    Ok(state)
}

fn record_overlaps_regions(
    record: &BamRecordView<'_>,
    regions: &[NormalizedRegion],
    path: &std::path::Path,
) -> Result<bool, AppError> {
    let Some((reference_index, start, end)) = mapped_record_interval(record, path)? else {
        return Ok(false);
    };
    Ok(regions.iter().any(|region| {
        region.reference_index == reference_index
            && start < region.end_0_based_exclusive
            && end > region.start_0_based
    }))
}

fn mapped_record_interval(
    record: &BamRecordView<'_>,
    path: &std::path::Path,
) -> Result<Option<(usize, u32, u32)>, AppError> {
    if record.flag_summary().is_unmapped || record.ref_id() < 0 || record.pos() < 0 {
        return Ok(None);
    }
    let reference_index =
        usize::try_from(record.ref_id()).map_err(|_| AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "Mapped BAM record reference id could not be represented as an index."
                .to_string(),
        })?;
    let start = record.pos() as u32;
    let span = reference_span(record.cigar_bytes(), path)?;
    let end = start
        .checked_add(span)
        .ok_or_else(|| AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "Mapped BAM record reference interval overflowed u32.".to_string(),
        })?;

    Ok(Some((reference_index, start, end)))
}

fn reference_span(cigar_bytes: &[u8], path: &std::path::Path) -> Result<u32, AppError> {
    if cigar_bytes.is_empty() {
        return Ok(1);
    }

    let mut span = 0_u32;
    for chunk in cigar_bytes.chunks_exact(4) {
        let raw = u32::from_le_bytes(chunk.try_into().expect("chunk size checked"));
        let op_len = raw >> 4;
        let op = raw & 0x0f;
        if matches!(op, 0 | 2 | 3 | 7 | 8) {
            span = span
                .checked_add(op_len)
                .ok_or_else(|| AppError::InvalidRecord {
                    path: path.to_path_buf(),
                    detail: "BAM CIGAR reference span overflowed u32.".to_string(),
                })?;
        }
    }

    Ok(span.max(1))
}

fn update_scan_state(state: &mut ScanState, record: &BamRecordView<'_>) {
    let flags = record.flag_summary();
    let ref_id = record.ref_id();
    let mapped = ref_id >= 0 && !flags.is_unmapped;
    let unmapped = flags.is_unmapped || ref_id < 0;
    let contradictory = (ref_id >= 0 && flags.is_unmapped) || (ref_id < 0 && !flags.is_unmapped);

    if contradictory {
        state.inconsistent_records_observed += 1;
    }

    if mapped {
        state.mapped_records_observed += 1;
        if let Ok(index) = usize::try_from(ref_id) {
            *state.mapped_per_reference.entry(index).or_insert(0) += 1;
        }
    }

    if unmapped {
        state.unmapped_records_observed += 1;
        if ref_id >= 0 {
            if let Ok(index) = usize::try_from(ref_id) {
                *state
                    .placed_unmapped_per_reference
                    .entry(index)
                    .or_insert(0) += 1;
            }
        }
    }
}

fn build_scan_payload(
    header: &HeaderPayload,
    index_info: IndexInfo,
    index_note: Option<String>,
    scan_state: ScanState,
    reached_eof: bool,
    full_scan: bool,
) -> CheckMapPayload {
    let references_with_mapped_reads_observed = scan_state.mapped_per_reference.len();
    let references = header
        .header
        .references
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            let mapped_reads = scan_state
                .mapped_per_reference
                .get(&index)
                .copied()
                .unwrap_or(0);
            let unmapped_reads = scan_state
                .placed_unmapped_per_reference
                .get(&index)
                .copied()
                .unwrap_or(0);

            ReferenceMappingInfo {
                name: reference.name.clone(),
                length: reference.length,
                mapped_reads: Some(mapped_reads),
                unmapped_reads: Some(unmapped_reads),
                observed: Some(mapped_reads > 0),
            }
        })
        .collect::<Vec<_>>();

    let (mapping_status, has_mapped_reads, confidence, note) = if scan_state.records_examined == 0 {
        (
            MappingStatus::Indeterminate,
            None,
            ConfidenceLevel::Low,
            "No alignment records were available to assess mapping state from the BAM stream.",
        )
    } else if scan_state.mapped_records_observed > 0 {
        (
            MappingStatus::Mapped,
            Some(true),
            if scan_state.inconsistent_records_observed == 0 {
                ConfidenceLevel::Medium
            } else {
                ConfidenceLevel::Low
            },
            if reached_eof || full_scan {
                "Mapping status is inferred from a scan of alignment records because no usable index was available."
            } else {
                "Mapping status is inferred from a bounded scan of alignment records because no usable index was available."
            },
        )
    } else if reached_eof || full_scan {
        (
            MappingStatus::Unmapped,
            Some(false),
            if scan_state.inconsistent_records_observed == 0 {
                ConfidenceLevel::High
            } else {
                ConfidenceLevel::Low
            },
            "No mapped alignments were observed in the scanned alignment stream. This is not a statement about index validity or full BAM validation.",
        )
    } else {
        (
            MappingStatus::Unmapped,
            Some(false),
            if scan_state.inconsistent_records_observed == 0 {
                ConfidenceLevel::Medium
            } else {
                ConfidenceLevel::Low
            },
            "No mapped alignments were observed in the bounded scan. This is not a full-file validation unless full-scan mode is used.",
        )
    };

    let semantic_note = match index_note {
        Some(extra) => format!("{note} {extra}"),
        None => note.to_string(),
    };

    CheckMapPayload {
        format: "BAM",
        mapping_status,
        has_mapped_reads,
        evidence_source: EvidenceSource::Scan,
        index: index_info,
        references,
        region_scope: None,
        summary: MappingSummary {
            references_defined: header.header.references.len(),
            references_with_mapped_reads: None,
            total_mapped_reads: None,
            total_unmapped_reads: None,
            records_examined: Some(scan_state.records_examined),
            mapped_records_observed: Some(scan_state.mapped_records_observed),
            unmapped_records_observed: Some(scan_state.unmapped_records_observed),
            references_with_mapped_reads_observed: Some(references_with_mapped_reads_observed),
            inconsistent_records_observed: Some(scan_state.inconsistent_records_observed),
            region_records_examined: None,
            region_mapped_records_observed: None,
            region_unmapped_records_observed: None,
        },
        confidence,
        semantic_note,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, thread, time::Duration};

    use crate::{
        bam::index::{build_bai_index_from_bam, test_support::build_bai_file, write_bai_index},
        formats::bgzf::test_support::{
            build_bam_file_with_header, build_bam_file_with_header_and_records, build_light_record,
            write_temp_file,
        },
    };

    use super::{CheckMapRequest, EvidenceSource, MappingStatus, run};

    fn region_values(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn uses_bai_counts_when_available() {
        let bam_path = write_temp_file(
            "check-map-index",
            "bam",
            &build_bam_file_with_header("@SQ\tSN:chr1\tLN:1000\n", &[("chr1", 1000)]),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        fs::write(&bai_path, build_bai_file(&[Some((5, 2))], Some(1)))
            .expect("bai fixture should be written");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Index));
        assert!(matches!(payload.mapping_status, MappingStatus::Mapped));
        assert!(payload.index.present);
        assert!(payload.index.used);
        assert_eq!(payload.summary.records_examined, None);
        assert_eq!(payload.references[0].mapped_reads, Some(5));
        assert_eq!(payload.references[0].unmapped_reads, Some(2));
        assert_eq!(payload.summary.total_mapped_reads, Some(5));
        assert_eq!(payload.summary.total_unmapped_reads, Some(3));
        assert!(
            payload
                .semantic_note
                .contains("derived from the BAM index and header")
        );
    }

    #[test]
    fn uses_generated_bai_sidecar_as_index_evidence() {
        let bam_path = write_temp_file(
            "check-map-generated-index",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(0, 10, "read1", 0),
                    build_light_record(-1, -1, "read2", 4),
                ],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        let index = build_bai_index_from_bam(&bam_path).expect("bai should build");
        write_bai_index(&bai_path, &index).expect("bai should write");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 1,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Index));
        assert!(payload.index.used);
        assert_eq!(payload.summary.records_examined, None);
        assert_eq!(payload.summary.total_mapped_reads, Some(1));
        assert_eq!(payload.summary.total_unmapped_reads, Some(1));
        assert!(
            payload
                .semantic_note
                .contains("passed structural validation")
        );
    }

    #[test]
    fn region_request_uses_indexed_traversal_when_bai_is_usable() {
        let bam_path = write_temp_file(
            "check-map-region-indexed",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(0, 5, "read1", 0),
                    build_light_record(0, 8, "read2", 0),
                    build_light_record(0, 50, "read3", 0),
                ],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        let index = build_bai_index_from_bam(&bam_path).expect("bai should build");
        write_bai_index(&bai_path, &index).expect("bai should write");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: region_values(&["chr1:6-9"]),
        })
        .expect("check_map region should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Index));
        assert!(payload.index.used);
        assert_eq!(payload.summary.records_examined, None);
        assert_eq!(payload.summary.total_mapped_reads, None);
        assert_eq!(payload.summary.region_records_examined, Some(2));
        assert_eq!(payload.summary.region_mapped_records_observed, Some(2));
        let region_scope = payload
            .region_scope
            .expect("region scope should be reported");
        assert!(matches!(
            region_scope.execution,
            super::RegionExecution::Indexed
        ));
        assert!(region_scope.index_path.is_some());
        assert!(region_scope.chunks_traversed.unwrap_or(0) > 0);
    }

    #[test]
    fn region_request_deduplicates_overlapping_intervals() {
        let bam_path = write_temp_file(
            "check-map-region-overlap",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(0, 5, "read1", 0),
                    build_light_record(0, 8, "read2", 0),
                ],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        let index = build_bai_index_from_bam(&bam_path).expect("bai should build");
        write_bai_index(&bai_path, &index).expect("bai should write");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: region_values(&["chr1:6-9", "chr1:8-9"]),
        })
        .expect("check_map region should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert_eq!(payload.summary.region_records_examined, Some(2));
        assert_eq!(payload.summary.region_mapped_records_observed, Some(2));
        assert_eq!(
            payload
                .region_scope
                .expect("region scope should be reported")
                .regions
                .len(),
            2
        );
    }

    #[test]
    fn region_request_falls_back_to_scan_without_index() {
        let bam_path = write_temp_file(
            "check-map-region-scan",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(0, 5, "read1", 0),
                    build_light_record(0, 50, "read2", 0),
                ],
            ),
        );

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: region_values(&["chr1:6-9"]),
        })
        .expect("check_map region should scan fallback");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(!payload.index.present);
        assert_eq!(payload.summary.region_records_examined, Some(1));
        let region_scope = payload
            .region_scope
            .expect("region scope should be reported");
        assert!(matches!(
            region_scope.execution,
            super::RegionExecution::ScanFallback
        ));
        assert_eq!(region_scope.fallback_mode, Some("native_scan_required"));
        assert_eq!(region_scope.scan_records_limit, Some(10));
    }

    #[test]
    fn region_request_unknown_reference_fails_deterministically() {
        let bam_path = write_temp_file(
            "check-map-region-unknown-reference",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 5, "read1", 0)],
            ),
        );

        let error = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: region_values(&["missing:1-10"]),
        })
        .expect_err("unknown reference should fail");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        assert_eq!(error.to_json_error().code, "invalid_region");
    }

    #[test]
    fn region_request_empty_interval_fails_deterministically() {
        let bam_path = write_temp_file(
            "check-map-region-empty",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 5, "read1", 0)],
            ),
        );

        let error = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: region_values(&["chr1:10-9"]),
        })
        .expect_err("empty interval should fail");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        assert_eq!(error.to_json_error().code, "invalid_region");
    }

    #[test]
    fn stale_bai_falls_back_to_scan() {
        let bam_path = write_temp_file(
            "check-map-stale-index",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 10, "read1", 0)],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        let index = build_bai_index_from_bam(&bam_path).expect("bai should build");
        write_bai_index(&bai_path, &index).expect("bai should write");
        thread::sleep(Duration::from_millis(1100));
        fs::OpenOptions::new()
            .append(true)
            .open(&bam_path)
            .expect("bam should open")
            .write_all(b"x")
            .expect("bam mtime should update");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(payload.index.present);
        assert!(!payload.index.used);
        assert_eq!(payload.summary.records_examined, Some(1));
        assert!(payload.semantic_note.contains("timestamp-stale"));
    }

    #[test]
    fn region_request_stale_bai_falls_back_to_scan() {
        let bam_path = write_temp_file(
            "check-map-region-stale-index",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 5, "read1", 0)],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        let index = build_bai_index_from_bam(&bam_path).expect("bai should build");
        write_bai_index(&bai_path, &index).expect("bai should write");
        thread::sleep(Duration::from_millis(1100));
        fs::OpenOptions::new()
            .append(true)
            .open(&bam_path)
            .expect("bam should open")
            .write_all(b"x")
            .expect("bam mtime should update");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: region_values(&["chr1:6-9"]),
        })
        .expect("check_map region should scan fallback");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(payload.index.present);
        assert!(!payload.index.used);
        assert_eq!(payload.summary.region_records_examined, Some(1));
        assert!(matches!(
            payload
                .region_scope
                .expect("region scope should be reported")
                .execution,
            super::RegionExecution::ScanFallback
        ));
    }

    #[test]
    fn falls_back_to_scan_without_index() {
        let bam_path = write_temp_file(
            "check-map-scan",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(0, 10, "read1", 0),
                    build_light_record(-1, -1, "read2", 4),
                ],
            ),
        );

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(matches!(payload.mapping_status, MappingStatus::Mapped));
        assert!(!payload.index.present);
        assert!(!payload.index.used);
        assert_eq!(payload.summary.records_examined, Some(2));
        assert_eq!(payload.summary.mapped_records_observed, Some(1));
        assert_eq!(payload.summary.unmapped_records_observed, Some(1));
        assert_eq!(payload.references[0].observed, Some(true));
        assert!(
            payload
                .semantic_note
                .contains("no usable index was available")
        );
    }

    #[test]
    fn incomplete_bai_metadata_falls_back_to_scan_with_index_note() {
        let bam_path = write_temp_file(
            "check-map-incomplete-index",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(-1, -1, "read1", 4),
                    build_light_record(0, 10, "read2", 0),
                ],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        fs::write(&bai_path, build_bai_file(&[None], None)).expect("bai fixture should be written");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(payload.index.present);
        assert!(!payload.index.used);
        assert_eq!(payload.summary.records_examined, Some(2));
        assert_eq!(payload.summary.mapped_records_observed, Some(1));
        assert!(
            payload
                .semantic_note
                .contains("per-reference mapped/unmapped metadata was incomplete")
        );
    }

    #[test]
    fn invalid_bai_falls_back_to_scan_with_unusable_index_note() {
        let bam_path = write_temp_file(
            "check-map-invalid-index",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 10, "read1", 0)],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        let mut invalid_bai = Vec::new();
        invalid_bai.extend_from_slice(b"BAI\x01");
        invalid_bai.extend_from_slice(&2_i32.to_le_bytes());
        fs::write(&bai_path, invalid_bai).expect("invalid bai fixture should be written");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(payload.index.present);
        assert!(!payload.index.used);
        assert_eq!(payload.summary.records_examined, Some(1));
        assert!(
            payload
                .semantic_note
                .contains("BAI index was present but unusable")
        );
    }

    #[test]
    fn prefer_index_false_forces_scan_even_when_bai_is_present() {
        let bam_path = write_temp_file(
            "check-map-index-disabled",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 10, "read1", 0)],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        fs::write(&bai_path, build_bai_file(&[Some((99, 0))], None))
            .expect("bai fixture should be written");

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: false,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(!payload.index.present);
        assert!(!payload.index.used);
        assert_eq!(payload.summary.mapped_records_observed, Some(1));
        assert_eq!(payload.summary.total_mapped_reads, None);
        assert!(
            payload
                .semantic_note
                .contains("Index preference was disabled")
        );
    }

    #[test]
    fn bounded_scan_can_report_apparently_unmapped() {
        let bam_path = write_temp_file(
            "check-map-unmapped",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(-1, -1, "read1", 4)],
            ),
        );

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(matches!(payload.mapping_status, MappingStatus::Unmapped));
        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert_eq!(payload.has_mapped_reads, Some(false));
        assert_eq!(payload.summary.records_examined, Some(1));
        assert_eq!(payload.summary.unmapped_records_observed, Some(1));
        assert!(
            payload
                .semantic_note
                .contains("No mapped alignments were observed in the scanned alignment stream")
        );
    }

    #[test]
    fn bounded_scan_caveat_before_later_mapped_record() {
        let bam_path = write_temp_file(
            "check-map-bounded-caveat",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(-1, -1, "read1", 4),
                    build_light_record(0, 10, "read2", 0),
                ],
            ),
        );

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 1,
            full_scan: false,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(matches!(payload.mapping_status, MappingStatus::Unmapped));
        assert_eq!(payload.has_mapped_reads, Some(false));
        assert_eq!(payload.summary.records_examined, Some(1));
        assert_eq!(payload.summary.mapped_records_observed, Some(0));
        assert!(
            payload
                .semantic_note
                .contains("No mapped alignments were observed in the bounded scan")
        );
    }

    #[test]
    fn full_scan_finds_mapped_record_after_bounded_window() {
        let bam_path = write_temp_file(
            "check-map-full-scan",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(-1, -1, "read1", 4),
                    build_light_record(0, 10, "read2", 0),
                ],
            ),
        );

        let payload = run(CheckMapRequest {
            bam: bam_path.clone(),
            sample_records: 1,
            full_scan: true,
            prefer_index: true,
            regions: Vec::new(),
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(matches!(payload.mapping_status, MappingStatus::Mapped));
        assert_eq!(payload.has_mapped_reads, Some(true));
        assert_eq!(payload.summary.records_examined, Some(2));
        assert_eq!(payload.summary.mapped_records_observed, Some(1));
        assert_eq!(payload.summary.unmapped_records_observed, Some(1));
    }
}
