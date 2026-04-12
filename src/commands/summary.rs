use std::{collections::HashSet, path::PathBuf};

use serde::Serialize;

use crate::{
    bam::{
        header::{HeaderPayload, parse_bam_header_from_reader},
        index::{BaiIndexSummary, IndexKind, IndexResolution, parse_bai, resolve_index_for_bam},
        reader::BamReader,
        records::read_next_light_record,
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

    let mut reader = match BamReader::open(&request.bam) {
        Ok(reader) => reader,
        Err(error) => {
            return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
        }
    };
    let header = match parse_bam_header_from_reader(&mut reader) {
        Ok(header) => header,
        Err(error) => {
            return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
        }
    };

    let index_summary = if request.prefer_index {
        match attempt_index_summary(&request.bam, header.header.references.len()) {
            Ok(summary) => summary,
            Err(error) => {
                return CommandResponse::failure("summary", Some(request.bam.as_path()), error);
            }
        }
    } else {
        None
    };

    let scan_result = match scan_summary_records(&mut reader, &request) {
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

    let payload = build_payload(&header, index_summary, scan_result, &request);
    CommandResponse::success("summary", Some(request.bam.as_path()), payload)
}

fn attempt_index_summary(
    bam_path: &std::path::Path,
    references_defined: usize,
) -> Result<Option<IndexSummaryUse>, AppError> {
    match resolve_index_for_bam(bam_path) {
        IndexResolution::Present(resolved) => match parse_bai(&resolved.path, references_defined) {
            Ok(summary) if summary.reference_summaries.iter().all(Option::is_some) => {
                Ok(Some(IndexSummaryUse {
                    kind: resolved.kind,
                    summary,
                }))
            }
            Ok(_) => Ok(None),
            Err(AppError::InvalidIndex { .. } | AppError::UnsupportedIndex { .. }) => Ok(None),
            Err(error) => Err(error),
        },
        IndexResolution::Unsupported(_) | IndexResolution::NotFound => Ok(None),
    }
}

struct ScanResult {
    snapshot: SummarySnapshot,
    reached_eof: bool,
}

fn scan_summary_records(
    reader: &mut BamReader,
    request: &SummaryRequest,
) -> Result<ScanResult, String> {
    let mut accumulator = SummaryAccumulator::new(request.include_mapq_hist);
    let record_limit = if request.full_scan {
        u64::MAX
    } else {
        request.sample_records.max(1) as u64
    };
    let mut reached_eof = false;

    while accumulator.snapshot().records_examined < record_limit {
        match read_next_light_record(reader) {
            Ok(Some(record)) => accumulator.observe(&record),
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
    })
}

fn build_payload(
    header: &HeaderPayload,
    index_summary: Option<IndexSummaryUse>,
    scan_result: ScanResult,
    request: &SummaryRequest,
) -> SummaryPayload {
    let full_file_scanned = scan_result.reached_eof;
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

    let semantic_note = if full_file_scanned {
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
        anomalies: Some(anomalies),
        flag_categories,
        index_derived,
        confidence: Some(confidence),
        semantic_note: Some(semantic_note),
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
            note: Some(
                "Index-derived mapped/unmapped totals are reported separately and do not replace scan-derived flag-category counts."
                    .to_string(),
            ),
        }
    })
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

fn fraction(value: u64, denominator: f64) -> Option<f64> {
    (denominator > 0.0).then_some(value as f64 / denominator)
}
