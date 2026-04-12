use std::{collections::HashMap, path::PathBuf};

use crate::{
    bam::{
        header::{HeaderPayload, parse_bam_header_from_reader},
        index::{BaiIndexSummary, IndexKind, IndexResolution, parse_bai, resolve_index_for_bam},
        reader::BamReader,
        records::{LightAlignmentRecord, read_next_light_record},
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
}

#[derive(Debug, Serialize)]
pub struct CheckMapPayload {
    pub format: &'static str,
    pub mapping_status: MappingStatus,
    pub has_mapped_reads: Option<bool>,
    pub evidence_source: EvidenceSource,
    pub index: IndexInfo,
    pub references: Vec<ReferenceMappingInfo>,
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

    let mut reader = BamReader::open(&request.bam)?;
    let header = parse_bam_header_from_reader(&mut reader)?;
    let references_defined = header.header.references.len();

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
            &header,
            index_info,
            index_summary,
            index_note,
        ));
    }

    let (scan_state, reached_eof) = scan_mapping_records(
        &mut reader,
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
        &header,
        index_info,
        index_note,
        scan_state,
        reached_eof,
        request.full_scan,
    ))
}

fn attempt_index_summary(
    bam_path: &std::path::Path,
    references_defined: usize,
) -> Result<(IndexInfo, Option<String>, Option<BaiIndexSummary>), AppError> {
    match resolve_index_for_bam(bam_path) {
        IndexResolution::Present(resolved) => match parse_bai(&resolved.path, references_defined) {
            Ok(summary) if summary.reference_summaries.iter().all(Option::is_some) => Ok((
                IndexInfo {
                    present: true,
                    kind: Some(resolved.kind),
                    used: true,
                },
                None,
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
        },
        IndexResolution::Unsupported(resolved) => Ok((
            IndexInfo {
                present: true,
                kind: Some(resolved.kind),
                used: false,
            },
            Some("CSI index detected, but CSI parsing is not implemented in this slice; falling back to alignment scan.".to_string()),
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
        },
        confidence: ConfidenceLevel::High,
        semantic_note,
    }
}

fn scan_mapping_records(
    reader: &mut BamReader,
    sample_records: usize,
    full_scan: bool,
) -> Result<(ScanState, bool), AppError> {
    let mut state = ScanState::default();
    let mut reached_eof = false;

    loop {
        if !full_scan && state.records_examined >= sample_records {
            break;
        }

        match read_next_light_record(reader)? {
            Some(record) => {
                state.records_examined += 1;
                update_scan_state(&mut state, &record);
            }
            None => {
                reached_eof = true;
                break;
            }
        }
    }

    Ok((state, reached_eof))
}

fn update_scan_state(state: &mut ScanState, record: &LightAlignmentRecord) {
    let mapped = record.ref_id >= 0 && !record.is_unmapped;
    let unmapped = record.is_unmapped || record.ref_id < 0;
    let contradictory =
        (record.ref_id >= 0 && record.is_unmapped) || (record.ref_id < 0 && !record.is_unmapped);

    if contradictory {
        state.inconsistent_records_observed += 1;
    }

    if mapped {
        state.mapped_records_observed += 1;
        if let Ok(index) = usize::try_from(record.ref_id) {
            *state.mapped_per_reference.entry(index).or_insert(0) += 1;
        }
    }

    if unmapped {
        state.unmapped_records_observed += 1;
        if record.ref_id >= 0 {
            if let Ok(index) = usize::try_from(record.ref_id) {
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
        },
        confidence,
        semantic_note,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bam::index::test_support::build_bai_file,
        formats::bgzf::test_support::{
            build_bam_file_with_header, build_bam_file_with_header_and_records, build_light_record,
            write_temp_file,
        },
    };

    use super::{CheckMapRequest, EvidenceSource, MappingStatus, run};

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
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");
        fs::remove_file(&bai_path).expect("bai fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Index));
        assert!(matches!(payload.mapping_status, MappingStatus::Mapped));
        assert_eq!(payload.summary.total_mapped_reads, Some(5));
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
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(matches!(payload.evidence_source, EvidenceSource::Scan));
        assert!(matches!(payload.mapping_status, MappingStatus::Mapped));
        assert_eq!(payload.summary.mapped_records_observed, Some(1));
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
        })
        .expect("check_map should succeed");

        fs::remove_file(&bam_path).expect("bam fixture should be removable");

        assert!(matches!(payload.mapping_status, MappingStatus::Unmapped));
        assert_eq!(payload.has_mapped_reads, Some(false));
    }
}
