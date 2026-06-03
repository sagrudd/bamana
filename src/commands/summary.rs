use std::{
    collections::HashSet,
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::{
    bam::{
        header::HeaderPayload,
        index::{
            BaiIndexSummary, IndexKind, IndexResolution, bam_newer_than_index, parse_bai,
            parse_bai_index, resolve_index_for_bam,
        },
        record::BamRecordView,
        region::{NormalizedRegion, NormalizedRegionSet, normalize_region_strings},
        region_plan::plan_bai_region_chunks,
        region_traversal::{RegionMatchedRecord, traverse_planned_region_chunks},
        scan::{BamScanner, LiveSummaryBamRecord, SummaryBamRecord},
        summary::{SummaryAccumulator, SummarySnapshot},
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct SummaryRequest {
    pub bam: PathBuf,
    pub sample_records: usize,
    pub full_scan: bool,
    pub prefer_index: bool,
    pub include_mapq_hist: bool,
    pub include_flags: bool,
    pub regions: Vec<String>,
    pub live_progress: bool,
    pub allow_incomplete: bool,
}

#[derive(Debug, Serialize)]
pub struct SummaryPayload {
    pub format: &'static str,
    pub mode: SummaryMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<SummaryEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<HeaderSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<ReferenceSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<RecordCountSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractions: Option<FractionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractions_observed: Option<FractionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapq: Option<MapqSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapping: Option<SummaryMappingInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_scope: Option<RegionScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anomalies: Option<AnomalySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_categories: Option<FlagCategorySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_derived: Option<IndexDerivedSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryMode {
    BoundedScan,
    FullScan,
    Indeterminate,
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
pub struct SummaryEvidence {
    pub header_used: bool,
    pub index_used: bool,
    pub records_scanned: u64,
    pub full_file_scanned: bool,
}

#[derive(Debug, Serialize)]
pub struct HeaderSummary {
    pub references_defined: usize,
    pub sort_order: Option<String>,
    pub sub_sort_order: Option<String>,
    pub group_order: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReferenceSummary {
    pub name: String,
    pub length: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapped_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmapped_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_mapped: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RecordCountSummary {
    pub records_examined: u64,
    pub records_total_known: Option<u64>,
    pub mapped_records: u64,
    pub unmapped_records: u64,
    pub primary_records: u64,
    pub secondary_records: u64,
    pub supplementary_records: u64,
    pub duplicate_records: u64,
    pub qc_fail_records: u64,
    pub paired_records: u64,
    pub properly_paired_records: u64,
    pub read1_records: u64,
    pub read2_records: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FractionSummary {
    pub fraction_mapped: Option<f64>,
    pub fraction_primary: Option<f64>,
    pub fraction_secondary: Option<f64>,
    pub fraction_supplementary: Option<f64>,
    pub fraction_duplicate: Option<f64>,
    pub fraction_qc_fail: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct MapqSummary {
    pub min: Option<u8>,
    pub max: Option<u8>,
    pub mean: Option<f64>,
    pub zero_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub histogram: Option<std::collections::BTreeMap<u8, u64>>,
}

#[derive(Debug, Serialize)]
pub struct SummaryMappingInfo {
    pub status: MappingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references_with_mapped_reads: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references_with_mapped_reads_observed: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MappingStatus {
    Mapped,
    Unmapped,
    Indeterminate,
}

#[derive(Debug, Serialize)]
pub struct AnomalySummary {
    pub contradictory_mapping_state_records: u64,
}

#[derive(Debug, Serialize)]
pub struct FlagCategorySummary {
    pub paired_records: u64,
    pub properly_paired_records: u64,
    pub secondary_records: u64,
    pub supplementary_records: u64,
    pub duplicate_records: u64,
    pub qc_fail_records: u64,
    pub read1_records: u64,
    pub read2_records: u64,
    pub reverse_strand_records: u64,
}

#[derive(Debug, Serialize)]
pub struct IndexDerivedSummary {
    pub present: bool,
    pub kind: Option<IndexKind>,
    pub used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_mapped_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_unmapped_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references_with_mapped_reads: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

struct IndexSummaryUse {
    kind: IndexKind,
    summary: BaiIndexSummary,
    note: String,
}

pub fn run(request: SummaryRequest) -> CommandResponse<SummaryPayload> {
    let probe = match probe_path(&request.bam) {
        Ok(probe) => probe,
        Err(error) => {
            return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
        }
    };

    if probe.detected_format == DetectedFormat::Unknown {
        return CommandResponse::failure(
            "summary",
            Some(request.bam.as_path()),
            AppError::UnknownFormat {
                path: request.bam.clone(),
            },
        );
    }

    if probe.detected_format != DetectedFormat::Bam {
        return CommandResponse::failure(
            "summary",
            Some(request.bam.as_path()),
            AppError::NotBam {
                path: request.bam.clone(),
                detected_format: probe.detected_format,
            },
        );
    }

    if probe.container != ContainerKind::Bgzf {
        return CommandResponse::failure(
            "summary",
            Some(request.bam.as_path()),
            AppError::InvalidBam {
                path: request.bam.clone(),
                detail: "Input did not present a BGZF-compatible container header.".to_string(),
            },
        );
    }

    let mut scanner = match BamScanner::open(&request.bam) {
        Ok(scanner) => scanner,
        Err(error) => {
            return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
        }
    };

    if !request.regions.is_empty() {
        if request.live_progress || request.allow_incomplete {
            return CommandResponse::failure(
                "summary",
                Some(request.bam.as_path()),
                AppError::UnsupportedInputForCommand {
                    path: request.bam.clone(),
                    detail: "summary --live-progress and --allow-incomplete are only supported for whole-file scan evidence; remove --region to inspect a growing BAM."
                        .to_string(),
                },
            );
        }
        let regions = match normalize_region_strings(
            &request.regions,
            &scanner.header().header.references,
            &request.bam,
        ) {
            Ok(regions) => regions,
            Err(error) => {
                return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
            }
        };
        return run_region_summary(request, scanner, regions);
    }

    let (index_summary, index_note) = if request.prefer_index {
        match attempt_index_summary(&request.bam, scanner.header().header.references.len()) {
            Ok(summary) => summary,
            Err(error) => {
                return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
            }
        }
    } else {
        (
            None,
            Some(
                "Index preference was disabled; summary evidence uses native scan mode."
                    .to_string(),
            ),
        )
    };

    let scan_result = match scan_summary_records(&mut scanner, &request) {
        Ok(result) => result,
        Err(error) => {
            let payload = SummaryPayload {
                format: "BAM",
                mode: SummaryMode::Indeterminate,
                evidence: None,
                header: None,
                references: None,
                counts: None,
                fractions: None,
                fractions_observed: None,
                mapq: None,
                mapping: None,
                region_scope: None,
                anomalies: None,
                flag_categories: None,
                index_derived: None,
                confidence: None,
                semantic_note: None,
            };
            return CommandResponse::failure_with_data(
                "summary",
                Some(request.bam.as_path()),
                Some(payload),
                AppError::SummaryUncertainty {
                    path: request.bam.clone(),
                    detail: error,
                },
            );
        }
    };

    let payload = build_payload(
        scanner.header(),
        index_summary,
        index_note,
        scan_result,
        &request,
    );
    CommandResponse::success("summary", Some(request.bam.as_path()), payload)
}

fn attempt_index_summary(
    bam_path: &std::path::Path,
    references_defined: usize,
) -> Result<(Option<IndexSummaryUse>, Option<String>), AppError> {
    match resolve_index_for_bam(bam_path) {
        IndexResolution::Present(resolved) => {
            if bam_newer_than_index(bam_path, &resolved.path) == Some(true) {
                return Ok((
                    None,
                    Some("BAI index was present but timestamp-stale; summary fell back to native scan evidence.".to_string()),
                ));
            }

            match parse_bai(&resolved.path, references_defined) {
                Ok(summary) if summary.reference_summaries.iter().all(Option::is_some) => Ok((
                    Some(IndexSummaryUse {
                        kind: resolved.kind,
                        summary,
                        note: "Discovered BAI sidecar passed structural validation and supplied complete per-reference mapped/unmapped metadata.".to_string(),
                    }),
                    None,
                )),
                Ok(_) => Ok((
                    None,
                    Some("BAI index was present, but per-reference mapped/unmapped metadata was incomplete; summary used native scan evidence.".to_string()),
                )),
                Err(AppError::InvalidIndex { detail, .. }) => Ok((
                    None,
                    Some(format!(
                        "BAI index was present but unusable: {detail} Summary used native scan evidence."
                    )),
                )),
                Err(AppError::UnsupportedIndex { detail, .. }) => Ok((
                    None,
                    Some(format!("{detail} Summary used native scan evidence.")),
                )),
                Err(error) => Err(error),
            }
        }
        IndexResolution::Unsupported(resolved) => Ok((
            None,
            Some(format!(
                "{} index detected, but it is not usable for summary index-derived evidence in this slice; summary used native scan evidence.",
                index_kind_label(resolved.kind)
            )),
        )),
        IndexResolution::NotFound => Ok((None, None)),
    }
}

fn run_region_summary(
    request: SummaryRequest,
    mut scanner: BamScanner,
    regions: NormalizedRegionSet,
) -> CommandResponse<SummaryPayload> {
    let references_defined = scanner.header().header.references.len();
    if request.prefer_index {
        match attempt_region_index_summary(&request, scanner.header(), references_defined, &regions)
        {
            Ok(Some(payload)) => {
                return CommandResponse::success("summary", Some(request.bam.as_path()), payload);
            }
            Ok(None) => {}
            Err(error) => {
                return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
            }
        }
    }

    let (index_present, index_kind, fallback_reason) =
        match region_fallback_index_context(&request, references_defined) {
            Ok(context) => context,
            Err(error) => {
                return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
            }
        };
    let scan_result = match scan_region_summary_records(&mut scanner, &request, &regions.regions) {
        Ok(result) => result,
        Err(error) => {
            return CommandResponse::failure(
                "summary",
                Some(request.bam.as_path()),
                AppError::SummaryUncertainty {
                    path: request.bam.clone(),
                    detail: error,
                },
            );
        }
    };
    let note = if scan_result.reached_eof || request.full_scan {
        "Region-scoped summary metrics are derived from native scan fallback and cannot be interpreted as full-file totals."
    } else {
        "Region-scoped summary metrics are derived from bounded native scan fallback and cannot be interpreted as full-file totals."
    };
    let note = match fallback_reason {
        Some(reason) => format!("{note} {reason}"),
        None => note.to_string(),
    };
    let payload = build_region_payload(
        scanner.header(),
        scan_result,
        &request,
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
            note,
        },
        IndexDerivedSummary {
            present: index_present,
            kind: index_kind,
            used: false,
            total_mapped_reads: None,
            total_unmapped_reads: None,
            references_with_mapped_reads: None,
            note: None,
        },
    );
    CommandResponse::success("summary", Some(request.bam.as_path()), payload)
}

fn attempt_region_index_summary(
    request: &SummaryRequest,
    header: &HeaderPayload,
    references_defined: usize,
    regions: &NormalizedRegionSet,
) -> Result<Option<SummaryPayload>, AppError> {
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
    let scan_result = scan_result_from_region_records(
        &traversal.records,
        request.include_mapq_hist,
        &request.bam,
    )?;
    Ok(Some(build_region_payload(
        header,
        scan_result,
        request,
        RegionScopeOptions {
            regions: regions.clone(),
            execution: RegionExecution::Indexed,
            index_path: Some(resolved.path.to_string_lossy().to_string()),
            fallback_mode: None,
            scan_records_limit: None,
            chunks_traversed: Some(traversal.chunks_traversed),
            raw_records_seen: Some(traversal.raw_records_seen),
            duplicate_records_suppressed: Some(traversal.duplicate_records_suppressed),
            note: "Region-scoped summary metrics are derived from validated indexed traversal and cannot be interpreted as full-file totals.".to_string(),
        },
        IndexDerivedSummary {
            present: true,
            kind: Some(resolved.kind),
            used: true,
            total_mapped_reads: None,
            total_unmapped_reads: None,
            references_with_mapped_reads: None,
            note: Some(
                "Index was used for region traversal only; whole-file BAI mapped/unmapped totals are intentionally omitted from region-scoped summary output."
                    .to_string(),
            ),
        },
    )))
}

fn region_fallback_index_context(
    request: &SummaryRequest,
    references_defined: usize,
) -> Result<(bool, Option<IndexKind>, Option<String>), AppError> {
    if !request.prefer_index {
        return Ok((
            false,
            None,
            Some("Index preference was disabled for region-scoped summary.".to_string()),
        ));
    }

    match resolve_index_for_bam(&request.bam) {
        IndexResolution::Present(resolved) => {
            if bam_newer_than_index(&request.bam, &resolved.path) == Some(true) {
                return Ok((
                    true,
                    Some(resolved.kind),
                    Some(
                        "BAI index was present but timestamp-stale for region traversal."
                            .to_string(),
                    ),
                ));
            }
            match parse_bai_index(&resolved.path, references_defined) {
                Ok(_) => Ok((
                    true,
                    Some(resolved.kind),
                    Some(
                        "BAI index was present but could not supply indexed region evidence; summary used native scan fallback."
                            .to_string(),
                    ),
                )),
                Err(AppError::UnsupportedIndex { detail, .. })
                | Err(AppError::InvalidIndex { detail, .. }) => Ok((
                    true,
                    Some(resolved.kind),
                    Some(format!(
                        "BAI index was present but unusable for region traversal: {detail}"
                    )),
                )),
                Err(error) => Err(error),
            }
        }
        IndexResolution::Unsupported(resolved) => Ok((
            true,
            Some(resolved.kind),
            Some(format!(
                "{} index detected, but it is not usable for region traversal.",
                index_kind_label(resolved.kind)
            )),
        )),
        IndexResolution::NotFound => Ok((
            false,
            None,
            Some("No usable BAM index was found for region traversal.".to_string()),
        )),
    }
}

struct ScanResult {
    snapshot: SummarySnapshot,
    reached_eof: bool,
    scanned_records: u64,
    stopped_at_incomplete_tail: bool,
}

struct RegionScopeOptions {
    regions: NormalizedRegionSet,
    execution: RegionExecution,
    index_path: Option<String>,
    fallback_mode: Option<&'static str>,
    scan_records_limit: Option<usize>,
    chunks_traversed: Option<usize>,
    raw_records_seen: Option<usize>,
    duplicate_records_suppressed: Option<usize>,
    note: String,
}

enum EitherSummaryRecord {
    Basic(SummaryBamRecord),
    Live(LiveSummaryBamRecord),
}

struct LiveProgressReporter {
    started_at: Instant,
    last_emit: Instant,
    sequence_bases: u128,
    quality_sum: u128,
    quality_count: u128,
}

impl LiveProgressReporter {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_emit: now - Duration::from_millis(500),
            sequence_bases: 0,
            quality_sum: 0,
            quality_count: 0,
        }
    }

    fn observe(
        &mut self,
        reads_parsed: u64,
        sequence_len: usize,
        quality_sum: u64,
        quality_count: u64,
    ) {
        self.sequence_bases += sequence_len as u128;
        self.quality_sum += quality_sum as u128;
        self.quality_count += quality_count as u128;
        if self.last_emit.elapsed() >= Duration::from_millis(500) {
            self.emit(reads_parsed, false, false);
        }
    }

    fn finish(&mut self, reads_parsed: u64, stopped_at_incomplete_tail: bool) {
        self.emit(reads_parsed, stopped_at_incomplete_tail, true);
    }

    fn emit(&mut self, reads_parsed: u64, stopped_at_incomplete_tail: bool, final_emit: bool) {
        self.last_emit = Instant::now();
        let elapsed = self.started_at.elapsed().as_secs_f64();
        let mean_length = if reads_parsed > 0 {
            format!("{:.1}", self.sequence_bases as f64 / reads_parsed as f64)
        } else {
            "NA".to_string()
        };
        let mean_q = if self.quality_count > 0 {
            format!("{:.2}", self.quality_sum as f64 / self.quality_count as f64)
        } else {
            "NA".to_string()
        };
        let reads_per_second = if elapsed > 0.0 {
            format!("{:.1}", reads_parsed as f64 / elapsed)
        } else {
            "NA".to_string()
        };
        let status = if stopped_at_incomplete_tail {
            "incomplete_tail"
        } else if final_emit {
            "complete"
        } else {
            "scanning"
        };
        eprint!(
            "\rstatus={status} reads_parsed={reads_parsed} reads_per_second={reads_per_second} mean_q={mean_q} mean_length={mean_length} elapsed={elapsed:.1}s"
        );
        let _ = io::stderr().flush();
        if final_emit {
            eprintln!();
        }
    }
}

fn scan_summary_records(
    scanner: &mut BamScanner,
    request: &SummaryRequest,
) -> Result<ScanResult, String> {
    let mut accumulator = SummaryAccumulator::new(request.include_mapq_hist);
    let mut live_progress = request.live_progress.then(LiveProgressReporter::new);
    let record_limit = if request.full_scan {
        u64::MAX
    } else {
        request.sample_records.max(1) as u64
    };
    let mut reached_eof = false;
    let mut scanned_records = 0;
    let mut stopped_at_incomplete_tail = false;

    while scanned_records < record_limit {
        let next_record = if live_progress.is_some() {
            scanner
                .next_live_summary_record()
                .map(|record| record.map(EitherSummaryRecord::Live))
        } else {
            scanner
                .next_summary_record()
                .map(|record| record.map(EitherSummaryRecord::Basic))
        };
        match next_record {
            Ok(Some(record)) => {
                scanned_records += 1;
                match record {
                    EitherSummaryRecord::Basic(record) => {
                        accumulator.observe_summary_record(record);
                    }
                    EitherSummaryRecord::Live(record) => {
                        accumulator.observe_summary_record(record.summary);
                        if let Some(reporter) = live_progress.as_mut() {
                            reporter.observe(
                                scanned_records,
                                record.sequence_len,
                                record.quality_sum,
                                record.quality_count,
                            );
                        }
                    }
                }
            }
            Ok(None) => {
                reached_eof = true;
                break;
            }
            Err(AppError::TruncatedFile { .. }) => {
                if request.allow_incomplete {
                    stopped_at_incomplete_tail = true;
                    break;
                }
                return Err(
                    "Alignment stream was truncated before a stable summary could be completed."
                        .to_string(),
                );
            }
            Err(AppError::InvalidRecord { detail, .. }) => return Err(detail),
            Err(error) => return Err(error.to_string()),
        }
    }
    if let Some(reporter) = live_progress.as_mut() {
        reporter.finish(scanned_records, stopped_at_incomplete_tail);
    }

    Ok(ScanResult {
        snapshot: accumulator.snapshot(),
        reached_eof,
        scanned_records,
        stopped_at_incomplete_tail,
    })
}

fn scan_region_summary_records(
    scanner: &mut BamScanner,
    request: &SummaryRequest,
    regions: &[NormalizedRegion],
) -> Result<ScanResult, String> {
    let mut accumulator = SummaryAccumulator::new(request.include_mapq_hist);
    let record_limit = if request.full_scan {
        u64::MAX
    } else {
        request.sample_records.max(1) as u64
    };
    let mut reached_eof = false;
    let mut scanned_records = 0;

    while scanned_records < record_limit {
        match scanner.next_record() {
            Ok(Some(record)) => {
                scanned_records += 1;
                if record_overlaps_regions(&record, regions).map_err(|error| error.to_string())? {
                    accumulator.observe_view(&record);
                }
            }
            Ok(None) => {
                reached_eof = true;
                break;
            }
            Err(AppError::TruncatedFile { .. }) => {
                return Err(
                    "Alignment stream was truncated before a stable summary could be completed."
                        .to_string(),
                );
            }
            Err(AppError::InvalidRecord { detail, .. }) => return Err(detail),
            Err(error) => return Err(error.to_string()),
        }
    }

    Ok(ScanResult {
        snapshot: accumulator.snapshot(),
        reached_eof,
        scanned_records,
        stopped_at_incomplete_tail: false,
    })
}

fn scan_result_from_region_records(
    records: &[RegionMatchedRecord],
    include_mapq_hist: bool,
    path: &std::path::Path,
) -> Result<ScanResult, AppError> {
    let mut accumulator = SummaryAccumulator::new(include_mapq_hist);
    for record in records {
        let view =
            BamRecordView::parse(&record.raw_record).map_err(|error| AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: error.detail().to_string(),
            })?;
        accumulator.observe_view(&view);
    }
    Ok(ScanResult {
        snapshot: accumulator.snapshot(),
        reached_eof: false,
        scanned_records: records.len() as u64,
        stopped_at_incomplete_tail: false,
    })
}

fn build_payload(
    header: &HeaderPayload,
    index_summary: Option<IndexSummaryUse>,
    index_note: Option<String>,
    scan_result: ScanResult,
    request: &SummaryRequest,
) -> SummaryPayload {
    let full_file_scanned = scan_result.reached_eof && !scan_result.stopped_at_incomplete_tail;
    let mode = if full_file_scanned {
        SummaryMode::FullScan
    } else {
        SummaryMode::BoundedScan
    };
    let records_total_known = full_file_scanned.then_some(scan_result.snapshot.records_examined);

    let header_summary = HeaderSummary {
        references_defined: header.header.references.len(),
        sort_order: header.header.hd.sort_order.clone(),
        sub_sort_order: header.header.hd.sub_sort_order.clone(),
        group_order: header.header.hd.group_order.clone(),
    };

    let index_derived = build_index_derived(index_summary.as_ref());
    let references = build_references(
        header,
        index_summary.as_ref(),
        &scan_result.snapshot.mapped_reference_ids,
    );

    let counts = RecordCountSummary {
        records_examined: scan_result.snapshot.records_examined,
        records_total_known,
        mapped_records: scan_result.snapshot.mapped_records,
        unmapped_records: scan_result.snapshot.unmapped_records,
        primary_records: scan_result.snapshot.primary_records,
        secondary_records: scan_result.snapshot.secondary_records,
        supplementary_records: scan_result.snapshot.supplementary_records,
        duplicate_records: scan_result.snapshot.duplicate_records,
        qc_fail_records: scan_result.snapshot.qc_fail_records,
        paired_records: scan_result.snapshot.paired_records,
        properly_paired_records: scan_result.snapshot.properly_paired_records,
        read1_records: scan_result.snapshot.read1_records,
        read2_records: scan_result.snapshot.read2_records,
    };

    let fraction_summary = build_fraction_summary(&counts);
    let mapq = MapqSummary {
        min: scan_result.snapshot.mapq_min,
        max: scan_result.snapshot.mapq_max,
        mean: (scan_result.snapshot.records_examined > 0).then_some(
            scan_result.snapshot.mapq_sum as f64 / scan_result.snapshot.records_examined as f64,
        ),
        zero_count: scan_result.snapshot.mapq_zero_count,
        histogram: scan_result.snapshot.mapq_histogram.clone(),
    };

    let mapping = build_mapping_summary(
        &scan_result.snapshot,
        index_summary.as_ref(),
        full_file_scanned,
    );
    let anomalies = AnomalySummary {
        contradictory_mapping_state_records: scan_result
            .snapshot
            .contradictory_mapping_state_records,
    };
    let flag_categories = request.include_flags.then_some(FlagCategorySummary {
        paired_records: scan_result.snapshot.paired_records,
        properly_paired_records: scan_result.snapshot.properly_paired_records,
        secondary_records: scan_result.snapshot.secondary_records,
        supplementary_records: scan_result.snapshot.supplementary_records,
        duplicate_records: scan_result.snapshot.duplicate_records,
        qc_fail_records: scan_result.snapshot.qc_fail_records,
        read1_records: scan_result.snapshot.read1_records,
        read2_records: scan_result.snapshot.read2_records,
        reverse_strand_records: scan_result.snapshot.reverse_strand_records,
    });

    let confidence = if scan_result.snapshot.records_examined == 0 {
        ConfidenceLevel::Low
    } else if full_file_scanned {
        ConfidenceLevel::High
    } else {
        ConfidenceLevel::Medium
    };

    let base_semantic_note = if full_file_scanned {
        if index_summary.is_some() {
            "Summary metrics are derived from a full alignment-record scan plus available header/index metadata.".to_string()
        } else {
            "Summary metrics are derived from a full alignment-record scan plus available header metadata.".to_string()
        }
    } else if index_summary.is_some() {
        "Summary metrics combine a bounded scan of alignment records with available header/index metadata. Scan-derived counts are observed rather than guaranteed full-file totals.".to_string()
    } else {
        "Summary metrics are derived from a bounded scan of alignment records and available header metadata; they may not represent full-file totals.".to_string()
    };
    let semantic_note = match index_note {
        Some(note) => format!("{base_semantic_note} {note}"),
        None => base_semantic_note,
    };
    let semantic_note = if scan_result.stopped_at_incomplete_tail {
        format!(
            "{semantic_note} Scan stopped at an incomplete trailing BGZF member or BAM record; reported metrics describe the complete records parsed before the growing-file boundary."
        )
    } else {
        semantic_note
    };

    SummaryPayload {
        format: "BAM",
        mode,
        evidence: Some(SummaryEvidence {
            header_used: true,
            index_used: index_summary.is_some(),
            records_scanned: scan_result.snapshot.records_examined,
            full_file_scanned,
        }),
        header: Some(header_summary),
        references: Some(references),
        counts: Some(counts),
        fractions: full_file_scanned.then_some(fraction_summary.clone()),
        fractions_observed: (!full_file_scanned).then_some(fraction_summary),
        mapq: Some(mapq),
        mapping: Some(mapping),
        region_scope: None,
        anomalies: Some(anomalies),
        flag_categories,
        index_derived,
        confidence: Some(confidence),
        semantic_note: Some(semantic_note),
    }
}

fn build_region_payload(
    header: &HeaderPayload,
    scan_result: ScanResult,
    request: &SummaryRequest,
    options: RegionScopeOptions,
    index_derived: IndexDerivedSummary,
) -> SummaryPayload {
    let mode = if scan_result.reached_eof || request.full_scan {
        SummaryMode::FullScan
    } else {
        SummaryMode::BoundedScan
    };
    let header_summary = HeaderSummary {
        references_defined: header.header.references.len(),
        sort_order: header.header.hd.sort_order.clone(),
        sub_sort_order: header.header.hd.sub_sort_order.clone(),
        group_order: header.header.hd.group_order.clone(),
    };
    let references = build_references(header, None, &scan_result.snapshot.mapped_reference_ids);
    let counts = RecordCountSummary {
        records_examined: scan_result.snapshot.records_examined,
        records_total_known: None,
        mapped_records: scan_result.snapshot.mapped_records,
        unmapped_records: scan_result.snapshot.unmapped_records,
        primary_records: scan_result.snapshot.primary_records,
        secondary_records: scan_result.snapshot.secondary_records,
        supplementary_records: scan_result.snapshot.supplementary_records,
        duplicate_records: scan_result.snapshot.duplicate_records,
        qc_fail_records: scan_result.snapshot.qc_fail_records,
        paired_records: scan_result.snapshot.paired_records,
        properly_paired_records: scan_result.snapshot.properly_paired_records,
        read1_records: scan_result.snapshot.read1_records,
        read2_records: scan_result.snapshot.read2_records,
    };
    let fraction_summary = build_fraction_summary(&counts);
    let mapq = MapqSummary {
        min: scan_result.snapshot.mapq_min,
        max: scan_result.snapshot.mapq_max,
        mean: (scan_result.snapshot.records_examined > 0).then_some(
            scan_result.snapshot.mapq_sum as f64 / scan_result.snapshot.records_examined as f64,
        ),
        zero_count: scan_result.snapshot.mapq_zero_count,
        histogram: scan_result.snapshot.mapq_histogram.clone(),
    };
    let mapping = build_mapping_summary(&scan_result.snapshot, None, false);
    let anomalies = AnomalySummary {
        contradictory_mapping_state_records: scan_result
            .snapshot
            .contradictory_mapping_state_records,
    };
    let flag_categories = request.include_flags.then_some(FlagCategorySummary {
        paired_records: scan_result.snapshot.paired_records,
        properly_paired_records: scan_result.snapshot.properly_paired_records,
        secondary_records: scan_result.snapshot.secondary_records,
        supplementary_records: scan_result.snapshot.supplementary_records,
        duplicate_records: scan_result.snapshot.duplicate_records,
        qc_fail_records: scan_result.snapshot.qc_fail_records,
        read1_records: scan_result.snapshot.read1_records,
        read2_records: scan_result.snapshot.read2_records,
        reverse_strand_records: scan_result.snapshot.reverse_strand_records,
    });
    let confidence = if scan_result.snapshot.records_examined == 0 {
        ConfidenceLevel::Low
    } else if matches!(options.execution, RegionExecution::Indexed)
        || scan_result.reached_eof
        || request.full_scan
    {
        ConfidenceLevel::High
    } else {
        ConfidenceLevel::Medium
    };
    SummaryPayload {
        format: "BAM",
        mode,
        evidence: Some(SummaryEvidence {
            header_used: true,
            index_used: matches!(options.execution, RegionExecution::Indexed),
            records_scanned: scan_result.scanned_records,
            full_file_scanned: false,
        }),
        header: Some(header_summary),
        references: Some(references),
        counts: Some(counts),
        fractions: None,
        fractions_observed: Some(fraction_summary),
        mapq: Some(mapq),
        mapping: Some(mapping),
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
        anomalies: Some(anomalies),
        flag_categories,
        index_derived: Some(index_derived),
        confidence: Some(confidence),
        semantic_note: Some(options.note),
    }
}

fn build_index_derived(index_summary: Option<&IndexSummaryUse>) -> Option<IndexDerivedSummary> {
    index_summary.map(|index_summary| {
        let mut total_mapped_reads = 0_u64;
        let mut total_unmapped_reads = index_summary.summary.unplaced_unmapped_reads.unwrap_or(0);
        let mut references_with_mapped_reads = 0_usize;

        for reference_summary in index_summary.summary.reference_summaries.iter().flatten() {
            total_mapped_reads += reference_summary.mapped_reads;
            total_unmapped_reads += reference_summary.unmapped_reads;
            if reference_summary.mapped_reads > 0 {
                references_with_mapped_reads += 1;
            }
        }

        IndexDerivedSummary {
            present: true,
            kind: Some(index_summary.kind),
            used: true,
            total_mapped_reads: Some(total_mapped_reads),
            total_unmapped_reads: Some(total_unmapped_reads),
            references_with_mapped_reads: Some(references_with_mapped_reads),
            note: Some(format!(
                "{} Index-derived mapped/unmapped totals are reported separately and do not replace scan-derived flag-category counts.",
                index_summary.note
            )),
        }
    })
}

fn index_kind_label(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Bai => "BAI",
        IndexKind::Csi => "CSI",
        IndexKind::Gzi => "GZI",
        IndexKind::Unknown => "UNKNOWN",
    }
}

fn build_references(
    header: &HeaderPayload,
    index_summary: Option<&IndexSummaryUse>,
    observed_mapped_reference_ids: &HashSet<usize>,
) -> Vec<ReferenceSummary> {
    header
        .header
        .references
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            let counts = index_summary
                .and_then(|summary| summary.summary.reference_summaries.get(index))
                .and_then(|entry| entry.as_ref());
            ReferenceSummary {
                name: reference.name.clone(),
                length: reference.length,
                mapped_reads: counts.map(|counts| counts.mapped_reads),
                unmapped_reads: counts.map(|counts| counts.unmapped_reads),
                observed_mapped: Some(observed_mapped_reference_ids.contains(&index)),
            }
        })
        .collect()
}

fn build_fraction_summary(counts: &RecordCountSummary) -> FractionSummary {
    let denominator = counts.records_examined as f64;
    FractionSummary {
        fraction_mapped: fraction(counts.mapped_records, denominator),
        fraction_primary: fraction(counts.primary_records, denominator),
        fraction_secondary: fraction(counts.secondary_records, denominator),
        fraction_supplementary: fraction(counts.supplementary_records, denominator),
        fraction_duplicate: fraction(counts.duplicate_records, denominator),
        fraction_qc_fail: fraction(counts.qc_fail_records, denominator),
    }
}

fn build_mapping_summary(
    snapshot: &SummarySnapshot,
    index_summary: Option<&IndexSummaryUse>,
    full_file_scanned: bool,
) -> SummaryMappingInfo {
    let index_references_with_mapped_reads = index_summary.map(|summary| {
        summary
            .summary
            .reference_summaries
            .iter()
            .flatten()
            .filter(|reference| reference.mapped_reads > 0)
            .count()
    });
    let index_mapped_reads = index_summary.map(|summary| {
        summary
            .summary
            .reference_summaries
            .iter()
            .flatten()
            .map(|reference| reference.mapped_reads)
            .sum::<u64>()
    });

    let status = if index_mapped_reads.unwrap_or(snapshot.mapped_records) > 0 {
        MappingStatus::Mapped
    } else if full_file_scanned || snapshot.records_examined > 0 {
        MappingStatus::Unmapped
    } else {
        MappingStatus::Indeterminate
    };

    SummaryMappingInfo {
        status,
        references_with_mapped_reads: full_file_scanned
            .then_some(snapshot.references_with_mapped_reads_observed)
            .or(index_references_with_mapped_reads),
        references_with_mapped_reads_observed: (!full_file_scanned)
            .then_some(snapshot.references_with_mapped_reads_observed),
    }
}

fn record_overlaps_regions(
    record: &BamRecordView<'_>,
    regions: &[NormalizedRegion],
) -> Result<bool, AppError> {
    let Some((reference_index, start, end)) = mapped_record_interval(record)? else {
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
) -> Result<Option<(usize, u32, u32)>, AppError> {
    if record.flag_summary().is_unmapped || record.ref_id() < 0 || record.pos() < 0 {
        return Ok(None);
    }
    let reference_index =
        usize::try_from(record.ref_id()).map_err(|_| AppError::InvalidRecord {
            path: PathBuf::from("<summary-region>"),
            detail: "Mapped BAM record reference id could not be represented as an index."
                .to_string(),
        })?;
    let start = record.pos() as u32;
    let span = reference_span(record.cigar_bytes())?;
    let end = start
        .checked_add(span)
        .ok_or_else(|| AppError::InvalidRecord {
            path: PathBuf::from("<summary-region>"),
            detail: "Mapped BAM record reference interval overflowed u32.".to_string(),
        })?;
    Ok(Some((reference_index, start, end)))
}

fn reference_span(cigar_bytes: &[u8]) -> Result<u32, AppError> {
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
                    path: PathBuf::from("<summary-region>"),
                    detail: "BAM CIGAR reference span overflowed u32.".to_string(),
                })?;
        }
    }

    Ok(span.max(1))
}

fn fraction(value: u64, denominator: f64) -> Option<f64> {
    (denominator > 0.0).then_some(value as f64 / denominator)
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, thread, time::Duration};

    use crate::{
        bam::index::{
            build_bai_index_from_bam,
            test_support::{build_bai_file, build_csi_header},
            write_bai_index,
        },
        bgzf::{
            BGZF_EOF_MARKER,
            test_support::{
                build_bam_file_with_header, build_bam_file_with_header_and_records,
                build_bgzf_member, build_light_record, write_temp_file,
            },
        },
    };

    use super::{ConfidenceLevel, MappingStatus, SummaryMode, SummaryRequest, run};

    fn region_values(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn header_only_full_summary_reports_empty_body_evidence() {
        let bam_path = write_temp_file(
            "summary-header-only",
            "bam",
            &build_bam_file_with_header("@SQ\tSN:chr1\tLN:1000\n", &[("chr1", 1000)]),
        );

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: true,
            prefer_index: true,
            include_mapq_hist: true,
            include_flags: true,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        assert!(matches!(payload.mode, SummaryMode::FullScan));
        assert!(matches!(payload.confidence, Some(ConfidenceLevel::Low)));
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(evidence.header_used);
        assert!(!evidence.index_used);
        assert_eq!(evidence.records_scanned, 0);
        assert!(evidence.full_file_scanned);
        let counts = payload.counts.expect("counts should be present");
        assert_eq!(counts.records_examined, 0);
        assert_eq!(counts.records_total_known, Some(0));
        assert_eq!(counts.mapped_records, 0);
        assert_eq!(counts.unmapped_records, 0);
        assert!(payload.fractions.is_some());
        assert!(payload.fractions_observed.is_none());
        assert!(payload.index_derived.is_none());
    }

    #[test]
    fn bounded_summary_uses_scanner_records() {
        let bam_path = write_temp_file(
            "summary-scanner-bounded",
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

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 1,
            full_scan: false,
            prefer_index: false,
            include_mapq_hist: true,
            include_flags: true,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        assert!(matches!(payload.mode, SummaryMode::BoundedScan));
        let evidence = payload.evidence.expect("evidence should be present");
        assert_eq!(evidence.records_scanned, 1);
        assert!(!evidence.full_file_scanned);
        let counts = payload.counts.expect("counts should be present");
        assert_eq!(counts.records_examined, 1);
        assert_eq!(counts.mapped_records, 1);
        assert_eq!(counts.unmapped_records, 0);
        assert_eq!(counts.records_total_known, None);
        assert!(payload.fractions.is_none());
        assert!(payload.fractions_observed.is_some());
        let mapping = payload.mapping.expect("mapping summary should be present");
        assert!(matches!(mapping.status, MappingStatus::Mapped));
        assert!(payload.flag_categories.is_some());
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("bounded scan")
        );
    }

    #[test]
    fn allow_incomplete_summary_reports_complete_prefix() {
        let mut bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:1000\n",
            &[("chr1", 1000)],
            &[build_light_record(0, 10, "read1", 0)],
        );
        bytes.truncate(bytes.len() - BGZF_EOF_MARKER.len());
        let mut trailing_member = build_bgzf_member(b"partially-written-tail");
        trailing_member.truncate(12);
        bytes.extend_from_slice(&trailing_member);
        let bam_path = write_temp_file("summary-growing-prefix", "bam", &bytes);

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: true,
            prefer_index: false,
            include_mapq_hist: false,
            include_flags: false,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: true,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert_eq!(evidence.records_scanned, 1);
        assert!(!evidence.full_file_scanned);
        assert_eq!(
            payload
                .counts
                .expect("counts should be present")
                .records_examined,
            1
        );
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("incomplete trailing")
        );
    }

    #[test]
    fn full_summary_reports_scanner_eof() {
        let bam_path = write_temp_file(
            "summary-scanner-full",
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

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 1,
            full_scan: true,
            prefer_index: false,
            include_mapq_hist: false,
            include_flags: false,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        assert!(matches!(payload.mode, SummaryMode::FullScan));
        let evidence = payload.evidence.expect("evidence should be present");
        assert_eq!(evidence.records_scanned, 2);
        assert!(evidence.full_file_scanned);
        let counts = payload.counts.expect("counts should be present");
        assert_eq!(counts.records_total_known, Some(2));
        assert_eq!(counts.mapped_records, 1);
        assert_eq!(counts.unmapped_records, 1);
        assert!(payload.fractions.is_some());
        assert!(payload.fractions_observed.is_none());
        assert!(payload.flag_categories.is_none());
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("full alignment-record scan")
        );
    }

    #[test]
    fn index_assisted_summary_keeps_index_totals_separate_from_scan_counts() {
        let bam_path = write_temp_file(
            "summary-index-assisted",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 10, "read1", 0)],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        fs::write(&bai_path, build_bai_file(&[Some((7, 3))], Some(2)))
            .expect("bai fixture should be written");

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(evidence.index_used);
        assert_eq!(evidence.records_scanned, 1);
        let counts = payload.counts.expect("counts should be present");
        assert_eq!(counts.records_examined, 1);
        assert_eq!(counts.mapped_records, 1);
        let index_derived = payload
            .index_derived
            .expect("index-derived summary should be present");
        assert!(index_derived.used);
        assert_eq!(index_derived.total_mapped_reads, Some(7));
        assert_eq!(index_derived.total_unmapped_reads, Some(5));
        let references = payload.references.expect("references should be present");
        assert_eq!(references[0].mapped_reads, Some(7));
        assert_eq!(references[0].unmapped_reads, Some(3));
        assert_eq!(references[0].observed_mapped, Some(true));
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("header/index metadata")
        );
    }

    #[test]
    fn generated_bai_sidecar_supplies_index_derived_summary() {
        let bam_path = write_temp_file(
            "summary-generated-index",
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

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 1,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(evidence.index_used);
        assert_eq!(evidence.records_scanned, 1);
        let index_derived = payload
            .index_derived
            .expect("index-derived summary should be present");
        assert!(index_derived.used);
        assert_eq!(index_derived.total_mapped_reads, Some(1));
        assert_eq!(index_derived.total_unmapped_reads, Some(1));
        assert!(
            index_derived
                .note
                .expect("index note should be present")
                .contains("passed structural validation")
        );
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("header/index metadata")
        );
    }

    #[test]
    fn region_summary_uses_indexed_traversal_when_bai_is_usable() {
        let bam_path = write_temp_file(
            "summary-region-indexed",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[
                    build_light_record(0, 5, "read1", 0),
                    build_light_record(0, 8, "read2", 1024),
                    build_light_record(0, 50, "read3", 0),
                ],
            ),
        );
        let bai_path = std::path::PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));
        let index = build_bai_index_from_bam(&bam_path).expect("bai should build");
        write_bai_index(&bai_path, &index).expect("bai should write");

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: true,
            include_flags: true,
            regions: region_values(&["chr1:6-9"]),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(evidence.index_used);
        assert_eq!(evidence.records_scanned, 2);
        let counts = payload.counts.expect("counts should be present");
        assert_eq!(counts.records_examined, 2);
        assert_eq!(counts.mapped_records, 2);
        assert_eq!(counts.records_total_known, None);
        assert!(payload.fractions.is_none());
        assert!(payload.fractions_observed.is_some());
        assert!(
            payload
                .mapq
                .expect("mapq should be present")
                .histogram
                .is_some()
        );
        assert_eq!(
            payload
                .flag_categories
                .expect("flag categories should be present")
                .duplicate_records,
            1
        );
        let region_scope = payload
            .region_scope
            .expect("region scope should be present");
        assert!(matches!(
            region_scope.execution,
            super::RegionExecution::Indexed
        ));
        assert!(region_scope.index_path.is_some());
        assert!(region_scope.chunks_traversed.unwrap_or(0) > 0);
        let index_derived = payload
            .index_derived
            .expect("index-derived status should be present");
        assert!(index_derived.used);
        assert_eq!(index_derived.total_mapped_reads, None);
    }

    #[test]
    fn region_summary_deduplicates_overlapping_intervals() {
        let bam_path = write_temp_file(
            "summary-region-overlap",
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

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: region_values(&["chr1:6-9", "chr1:8-9"]),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        assert_eq!(
            payload
                .counts
                .expect("counts should be present")
                .records_examined,
            2
        );
        assert_eq!(
            payload
                .region_scope
                .expect("region scope should be present")
                .regions
                .len(),
            2
        );
    }

    #[test]
    fn region_summary_falls_back_to_scan_without_index() {
        let bam_path = write_temp_file(
            "summary-region-scan",
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

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: region_values(&["chr1:6-9"]),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(!evidence.index_used);
        assert_eq!(evidence.records_scanned, 2);
        assert_eq!(
            payload
                .counts
                .expect("counts should be present")
                .records_examined,
            1
        );
        let region_scope = payload
            .region_scope
            .expect("region scope should be present");
        assert!(matches!(
            region_scope.execution,
            super::RegionExecution::ScanFallback
        ));
        assert_eq!(region_scope.fallback_mode, Some("native_scan_required"));
        assert_eq!(region_scope.scan_records_limit, Some(10));
    }

    #[test]
    fn region_summary_with_csi_header_reports_unsupported_scan_fallback() {
        let bam_path = write_temp_file(
            "summary-region-csi-fallback",
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
        let csi_path = std::path::PathBuf::from(format!("{}.csi", bam_path.to_string_lossy()));
        fs::write(&csi_path, build_csi_header(1)).expect("csi fixture should be written");

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: region_values(&["chr1:6-9"]),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&csi_path).expect("csi fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(!evidence.index_used);
        assert_eq!(
            payload
                .index_derived
                .as_ref()
                .expect("index context should be present")
                .kind,
            Some(crate::bam::index::IndexKind::Csi)
        );
        assert_eq!(
            payload
                .counts
                .expect("counts should be present")
                .records_examined,
            1
        );
        let region_scope = payload
            .region_scope
            .expect("region scope should be present");
        assert!(matches!(
            region_scope.execution,
            super::RegionExecution::ScanFallback
        ));
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("CSI index detected")
        );
    }

    #[test]
    fn region_summary_unknown_reference_fails_deterministically() {
        let bam_path = write_temp_file(
            "summary-region-unknown",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 5, "read1", 0)],
            ),
        );

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: region_values(&["missing:1-10"]),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error should be present").code,
            "invalid_region"
        );
    }

    #[test]
    fn region_summary_empty_interval_fails_deterministically() {
        let bam_path = write_temp_file(
            "summary-region-empty",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 5, "read1", 0)],
            ),
        );

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: region_values(&["chr1:10-9"]),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error should be present").code,
            "invalid_region"
        );
    }

    #[test]
    fn region_summary_rejects_growing_file_flags() {
        let bam_path = write_temp_file(
            "summary-region-growing-flags",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:1000\n",
                &[("chr1", 1000)],
                &[build_light_record(0, 5, "read1", 0)],
            ),
        );

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: region_values(&["chr1:1-10"]),
            live_progress: true,
            allow_incomplete: true,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error should be present").code,
            "unsupported_input_for_command"
        );
    }

    #[test]
    fn stale_bai_sidecar_falls_back_to_scan_summary() {
        let bam_path = write_temp_file(
            "summary-stale-index",
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

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(!evidence.index_used);
        assert_eq!(evidence.records_scanned, 1);
        assert!(payload.index_derived.is_none());
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("timestamp-stale")
        );
    }

    #[test]
    fn region_summary_stale_bai_falls_back_to_scan() {
        let bam_path = write_temp_file(
            "summary-region-stale-index",
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

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: true,
            include_mapq_hist: false,
            include_flags: false,
            regions: region_values(&["chr1:6-9"]),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(response.ok);
        let payload = response.data.expect("summary payload should be present");
        let evidence = payload.evidence.expect("evidence should be present");
        assert!(!evidence.index_used);
        assert_eq!(
            payload
                .counts
                .expect("counts should be present")
                .records_examined,
            1
        );
        assert!(matches!(
            payload
                .region_scope
                .expect("region scope should be present")
                .execution,
            super::RegionExecution::ScanFallback
        ));
        assert!(
            payload
                .semantic_note
                .expect("semantic note should be present")
                .contains("timestamp-stale")
        );
    }

    #[test]
    fn malformed_record_returns_indeterminate_failure_payload() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&5_i32.to_le_bytes());
        payload.extend_from_slice(b"chr1\0");
        payload.extend_from_slice(&1000_i32.to_le_bytes());
        payload.extend_from_slice(&(-1_i32).to_le_bytes());

        let mut bytes = build_bgzf_member(&payload);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let bam_path = write_temp_file("summary-malformed-record", "bam", &bytes);

        let response = run(SummaryRequest {
            bam: bam_path.clone(),
            sample_records: 10,
            full_scan: false,
            prefer_index: false,
            include_mapq_hist: false,
            include_flags: false,
            regions: Vec::new(),
            live_progress: false,
            allow_incomplete: false,
        });

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(!response.ok);
        let payload = response.data.expect("failure payload should be present");
        assert!(matches!(payload.mode, SummaryMode::Indeterminate));
        assert!(payload.evidence.is_none());
        let error = response.error.expect("summary error should be present");
        assert_eq!(error.code, "parse_uncertainty");
    }
}
