use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use crate::{
    bam::{
        record::BamRecordView,
        region::{NormalizedRegion, NormalizedRegionSet},
        region_plan::RegionChunkPlan,
        scan::{BamRecordVirtualOffsets, BamScanner},
    },
    error::AppError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionTraversalResult {
    pub bam_path: PathBuf,
    pub records: Vec<RegionMatchedRecord>,
    pub chunks_traversed: usize,
    pub raw_records_seen: usize,
    pub duplicate_records_suppressed: usize,
    pub fallback: RegionFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionMatchedRecord {
    pub raw_record: Vec<u8>,
    pub virtual_offsets: BamRecordVirtualOffsets,
    pub reference_index: usize,
    pub start_0_based: u32,
    pub end_0_based_exclusive: u32,
    pub matched_regions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionFallback {
    None,
    NativeScanRequired,
}

pub fn traverse_planned_region_chunks(
    bam_path: &Path,
    regions: &NormalizedRegionSet,
    plan: &RegionChunkPlan,
) -> Result<RegionTraversalResult, AppError> {
    if plan.total_coalesced_chunks == 0 {
        return Ok(RegionTraversalResult {
            bam_path: bam_path.to_path_buf(),
            records: Vec::new(),
            chunks_traversed: 0,
            raw_records_seen: 0,
            duplicate_records_suppressed: 0,
            fallback: RegionFallback::None,
        });
    }

    let grouped_regions = group_regions_by_reference(regions);
    let mut scanner = BamScanner::open(bam_path)?;
    let mut records_by_offset: BTreeMap<(u64, u64), RegionMatchedRecord> = BTreeMap::new();
    let mut chunks_traversed = 0;
    let mut raw_records_seen = 0;
    let mut duplicate_records_suppressed = 0;

    for reference_plan in &plan.reference_plans {
        let Some(reference_regions) = grouped_regions.get(&reference_plan.reference_index) else {
            continue;
        };

        for chunk in &reference_plan.coalesced_chunks {
            chunks_traversed += 1;
            let raw_records = scanner.raw_records_in_virtual_range(chunk.start, chunk.end)?;
            raw_records_seen += raw_records.len();

            for raw in raw_records {
                let view = BamRecordView::parse(&raw.raw_record).map_err(|error| {
                    AppError::InvalidRecord {
                        path: bam_path.to_path_buf(),
                        detail: error.detail().to_string(),
                    }
                })?;
                let Some((reference_index, start, end)) = mapped_record_interval(&view, bam_path)?
                else {
                    continue;
                };
                if reference_index != reference_plan.reference_index {
                    continue;
                }

                let matched_regions =
                    matching_regions(reference_regions, start, end).collect::<Vec<_>>();
                if matched_regions.is_empty() {
                    continue;
                }

                let key = (
                    raw.virtual_offsets.start.packed(),
                    raw.virtual_offsets.end.packed(),
                );
                if let Some(existing) = records_by_offset.get_mut(&key) {
                    duplicate_records_suppressed += 1;
                    merge_region_matches(&mut existing.matched_regions, matched_regions);
                } else {
                    records_by_offset.insert(
                        key,
                        RegionMatchedRecord {
                            raw_record: raw.raw_record,
                            virtual_offsets: raw.virtual_offsets,
                            reference_index,
                            start_0_based: start,
                            end_0_based_exclusive: end,
                            matched_regions,
                        },
                    );
                }
            }
        }
    }

    Ok(RegionTraversalResult {
        bam_path: bam_path.to_path_buf(),
        records: records_by_offset.into_values().collect(),
        chunks_traversed,
        raw_records_seen,
        duplicate_records_suppressed,
        fallback: RegionFallback::None,
    })
}

pub fn scan_fallback_required(path: &Path) -> RegionTraversalResult {
    RegionTraversalResult {
        bam_path: path.to_path_buf(),
        records: Vec::new(),
        chunks_traversed: 0,
        raw_records_seen: 0,
        duplicate_records_suppressed: 0,
        fallback: RegionFallback::NativeScanRequired,
    }
}

fn group_regions_by_reference(
    regions: &NormalizedRegionSet,
) -> BTreeMap<usize, Vec<&NormalizedRegion>> {
    let mut grouped: BTreeMap<usize, Vec<&NormalizedRegion>> = BTreeMap::new();
    for region in &regions.regions {
        grouped
            .entry(region.reference_index)
            .or_default()
            .push(region);
    }
    grouped
}

fn mapped_record_interval(
    record: &BamRecordView<'_>,
    path: &Path,
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

fn matching_regions<'a>(
    regions: &'a [&'a NormalizedRegion],
    start: u32,
    end: u32,
) -> impl Iterator<Item = String> + 'a {
    regions
        .iter()
        .filter(move |region| start < region.end_0_based_exclusive && end > region.start_0_based)
        .map(|region| region.original.clone())
}

fn merge_region_matches(existing: &mut Vec<String>, additional: Vec<String>) {
    let mut seen = existing.iter().cloned().collect::<BTreeSet<_>>();
    for region in additional {
        if seen.insert(region.clone()) {
            existing.push(region);
        }
    }
}

fn reference_span(cigar_bytes: &[u8], path: &Path) -> Result<u32, AppError> {
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

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bam::{
            index::{IndexKind, build_bai_index_from_bam},
            record::BamRecordView,
            region::normalize_region_strings,
            region_plan::plan_bai_region_chunks,
            region_traversal::{
                RegionFallback, scan_fallback_required, traverse_planned_region_chunks,
            },
        },
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    fn bam_fixture(name: &str) -> std::path::PathBuf {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(0, 8, "read2", 0);
        let third = build_light_record(1, 5, "read3", 0);
        let fourth = build_light_record(0, 50, "read4", 0);
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:100\n@SQ\tSN:chr2\tLN:100\n",
            &[("chr1", 100), ("chr2", 100)],
            &[first, second, fourth, third],
        );
        write_temp_file(name, "bam", &bytes)
    }

    fn region_strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn traverses_planned_chunks_and_filters_to_requested_region() {
        let path = bam_fixture("region-traversal-filter");
        let index = build_bai_index_from_bam(&path).expect("index should build");
        let references = vec![
            crate::bam::header::ReferenceRecord {
                name: "chr1".to_string(),
                length: 100,
                index: 0,
                header_fields: Default::default(),
                text_header_length: None,
            },
            crate::bam::header::ReferenceRecord {
                name: "chr2".to_string(),
                length: 100,
                index: 1,
                header_fields: Default::default(),
                text_header_length: None,
            },
        ];
        let regions = normalize_region_strings(&region_strings(&["chr1:6-9"]), &references, &path)
            .expect("region should normalize");
        let plan = plan_bai_region_chunks(
            &regions,
            &index,
            &path.with_extension("bam.bai"),
            IndexKind::Bai,
            false,
        )
        .expect("plan should build");

        let result =
            traverse_planned_region_chunks(&path, &regions, &plan).expect("traversal should work");
        let names = result
            .records
            .iter()
            .map(|record| {
                BamRecordView::parse(&record.raw_record)
                    .expect("record should parse")
                    .read_name()
                    .to_string()
            })
            .collect::<Vec<_>>();

        fs::remove_file(path).expect("fixture should be removable");
        assert_eq!(names, vec!["read1", "read2"]);
        assert_eq!(result.fallback, RegionFallback::None);
        assert!(result.chunks_traversed > 0);
    }

    #[test]
    fn does_not_double_count_overlapping_region_chunks() {
        let path = bam_fixture("region-traversal-deduplicate");
        let index = build_bai_index_from_bam(&path).expect("index should build");
        let references = vec![crate::bam::header::ReferenceRecord {
            name: "chr1".to_string(),
            length: 100,
            index: 0,
            header_fields: Default::default(),
            text_header_length: None,
        }];
        let regions = normalize_region_strings(
            &region_strings(&["chr1:6-9", "chr1:8-9"]),
            &references,
            &path,
        )
        .expect("regions should normalize");
        let plan = plan_bai_region_chunks(
            &regions,
            &index,
            &path.with_extension("bam.bai"),
            IndexKind::Bai,
            false,
        )
        .expect("plan should build");

        let result =
            traverse_planned_region_chunks(&path, &regions, &plan).expect("traversal should work");
        let names = result
            .records
            .iter()
            .map(|record| {
                BamRecordView::parse(&record.raw_record)
                    .expect("record should parse")
                    .read_name()
                    .to_string()
            })
            .collect::<Vec<_>>();

        fs::remove_file(path).expect("fixture should be removable");
        assert_eq!(names, vec!["read1", "read2"]);
        assert_eq!(
            result.records[1].matched_regions,
            vec!["chr1:6-9".to_string(), "chr1:8-9".to_string()]
        );
    }

    #[test]
    fn records_native_scan_fallback_when_no_usable_index_is_available() {
        let fallback = scan_fallback_required(std::path::Path::new("input.bam"));

        assert_eq!(fallback.fallback, RegionFallback::NativeScanRequired);
        assert!(fallback.records.is_empty());
    }
}
