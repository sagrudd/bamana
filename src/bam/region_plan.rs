use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use crate::{
    bam::{
        index::{BaiChunk, BaiIndex, IndexKind, bai_bins_for_region},
        region::{NormalizedRegion, NormalizedRegionSet},
    },
    bgzf::virtual_offset::VirtualOffset,
    error::AppError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionChunkPlan {
    pub index_path: PathBuf,
    pub index_kind: IndexKind,
    pub requested_regions: usize,
    pub reference_plans: Vec<ReferenceChunkPlan>,
    pub total_candidate_chunks: usize,
    pub total_coalesced_chunks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceChunkPlan {
    pub reference_name: String,
    pub reference_index: usize,
    pub requested_regions: Vec<String>,
    pub candidate_bins: Vec<u32>,
    pub candidate_chunks: Vec<BaiChunk>,
    pub coalesced_chunks: Vec<BaiChunk>,
}

pub fn plan_bai_region_chunks(
    regions: &NormalizedRegionSet,
    index: &BaiIndex,
    index_path: &Path,
    index_kind: IndexKind,
    stale: bool,
) -> Result<RegionChunkPlan, AppError> {
    if index_kind != IndexKind::Bai {
        return Err(AppError::UnsupportedIndex {
            path: index_path.to_path_buf(),
            detail: format!(
                "Indexed region chunk planning requires a validated BAI index; observed {index_kind:?}."
            ),
        });
    }
    if stale {
        return Err(AppError::MissingIndex {
            path: index_path.to_path_buf(),
            detail: Some(
                "The selected BAI sidecar is stale and cannot be used for indexed region evidence."
                    .to_string(),
            ),
        });
    }
    if regions.regions.is_empty() {
        return Err(AppError::InvalidRegion {
            path: index_path.to_path_buf(),
            detail: "At least one normalized region is required for chunk planning.".to_string(),
        });
    }

    let mut grouped: BTreeMap<usize, Vec<&NormalizedRegion>> = BTreeMap::new();
    for region in &regions.regions {
        if region.reference_index >= index.references.len() {
            return Err(AppError::InvalidIndex {
                path: index_path.to_path_buf(),
                detail: format!(
                    "Region '{}' resolved to reference index {}, but the BAI has {} references.",
                    region.original,
                    region.reference_index,
                    index.references.len()
                ),
            });
        }
        grouped
            .entry(region.reference_index)
            .or_default()
            .push(region);
    }

    let mut reference_plans = Vec::new();
    let mut total_candidate_chunks = 0;
    let mut total_coalesced_chunks = 0;

    for (reference_index, requested_regions) in grouped {
        let reference_index_data = &index.references[reference_index];
        let mut candidate_bins = BTreeSet::new();
        let mut requested = Vec::new();

        for region in &requested_regions {
            requested.push(region.original.clone());
            for bin in bai_bins_for_region(region.start_0_based, region.end_0_based_exclusive)? {
                candidate_bins.insert(bin);
            }
        }

        let candidate_bins = candidate_bins.into_iter().collect::<Vec<_>>();
        let mut candidate_chunks = Vec::new();
        for bin in &candidate_bins {
            if let Some(chunks) = reference_index_data.bins.get(bin) {
                for chunk in chunks {
                    validate_chunk(chunk, index_path)?;
                    candidate_chunks.push(chunk.clone());
                }
            }
        }

        candidate_chunks.sort_by_key(|chunk| (chunk.start, chunk.end));
        let coalesced_chunks = coalesce_chunks(&candidate_chunks, index_path)?;

        total_candidate_chunks += candidate_chunks.len();
        total_coalesced_chunks += coalesced_chunks.len();

        reference_plans.push(ReferenceChunkPlan {
            reference_name: requested_regions
                .first()
                .map(|region| region.reference_name.clone())
                .unwrap_or_default(),
            reference_index,
            requested_regions: requested,
            candidate_bins,
            candidate_chunks,
            coalesced_chunks,
        });
    }

    Ok(RegionChunkPlan {
        index_path: index_path.to_path_buf(),
        index_kind,
        requested_regions: regions.regions.len(),
        reference_plans,
        total_candidate_chunks,
        total_coalesced_chunks,
    })
}

fn coalesce_chunks(chunks: &[BaiChunk], path: &Path) -> Result<Vec<BaiChunk>, AppError> {
    let mut coalesced: Vec<BaiChunk> = Vec::new();
    for chunk in chunks {
        validate_chunk(chunk, path)?;
        if let Some(last) = coalesced.last_mut() {
            if chunk.start <= last.end {
                if chunk.end > last.end {
                    last.end = chunk.end;
                }
                continue;
            }
        }
        coalesced.push(chunk.clone());
    }
    Ok(coalesced)
}

fn validate_chunk(chunk: &BaiChunk, path: &Path) -> Result<(), AppError> {
    if chunk.start == VirtualOffset::ZERO && chunk.end == VirtualOffset::ZERO {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "BAI chunk had impossible zero start and end virtual offsets.".to_string(),
        });
    }
    if chunk.start >= chunk.end {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: format!(
                "BAI chunk start virtual offset {} was not before end virtual offset {}.",
                chunk.start.packed(),
                chunk.end.packed()
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::Path};

    use crate::{
        bam::{
            header::{ReferenceHeaderFields, ReferenceRecord},
            index::{BaiChunk, BaiIndex, BaiReferenceIndex, IndexKind, bai_bins_for_region},
            region::normalize_region_strings,
        },
        bgzf::virtual_offset::VirtualOffset,
        error::AppError,
    };

    use super::plan_bai_region_chunks;

    fn references() -> Vec<ReferenceRecord> {
        vec![
            reference("chr1", 1_000_000, 0),
            reference("chr2", 1_000_000, 1),
        ]
    }

    fn reference(name: &str, length: u32, index: usize) -> ReferenceRecord {
        ReferenceRecord {
            name: name.to_string(),
            length,
            index,
            header_fields: ReferenceHeaderFields::default(),
            text_header_length: None,
        }
    }

    fn chunk(start: u64, end: u64) -> BaiChunk {
        BaiChunk {
            start: VirtualOffset::from_packed(start),
            end: VirtualOffset::from_packed(end),
        }
    }

    fn reference_index(chunks: &[(u32, Vec<BaiChunk>)]) -> BaiReferenceIndex {
        BaiReferenceIndex {
            bins: chunks.iter().cloned().collect::<BTreeMap<_, _>>(),
            linear_index: Vec::new(),
            mapped_reads: 0,
            unmapped_reads: 0,
        }
    }

    fn index() -> BaiIndex {
        BaiIndex {
            references: vec![
                reference_index(&[
                    (0, vec![chunk(10, 20)]),
                    (4_681, vec![chunk(18, 30), chunk(40, 50)]),
                    (4_682, vec![chunk(60, 70)]),
                ]),
                reference_index(&[(4_681, vec![chunk(100, 110)])]),
            ],
            unplaced_unmapped_reads: 0,
        }
    }

    #[test]
    fn bai_query_bins_cover_all_hierarchy_levels() {
        let bins = bai_bins_for_region(0, 1).expect("bins should compute");

        assert_eq!(bins, vec![0, 1, 9, 73, 585, 4_681]);
    }

    #[test]
    fn plans_and_coalesces_chunks_deterministically() {
        let regions = normalize_region_strings(
            &["chr1:1-100".to_string()],
            &references(),
            Path::new("input.bam"),
        )
        .expect("region should normalize");

        let plan = plan_bai_region_chunks(
            &regions,
            &index(),
            Path::new("input.bam.bai"),
            IndexKind::Bai,
            false,
        )
        .expect("plan should build");

        assert_eq!(plan.requested_regions, 1);
        assert_eq!(plan.total_candidate_chunks, 3);
        assert_eq!(plan.total_coalesced_chunks, 2);
        assert_eq!(
            plan.reference_plans[0].candidate_bins,
            vec![0, 1, 9, 73, 585, 4_681]
        );
        assert_eq!(
            plan.reference_plans[0].coalesced_chunks,
            vec![chunk(10, 30), chunk(40, 50)]
        );
    }

    #[test]
    fn preserves_multi_reference_plans() {
        let regions = normalize_region_strings(
            &["chr2:1-10".to_string(), "chr1:20000-20010".to_string()],
            &references(),
            Path::new("input.bam"),
        )
        .expect("regions should normalize");

        let plan = plan_bai_region_chunks(
            &regions,
            &index(),
            Path::new("input.bam.bai"),
            IndexKind::Bai,
            false,
        )
        .expect("plan should build");

        assert_eq!(plan.reference_plans.len(), 2);
        assert_eq!(plan.reference_plans[0].reference_name, "chr1");
        assert_eq!(plan.reference_plans[1].reference_name, "chr2");
    }

    #[test]
    fn reports_no_hit_intervals_without_error() {
        let regions = normalize_region_strings(
            &["chr2:70000-70010".to_string()],
            &references(),
            Path::new("input.bam"),
        )
        .expect("region should normalize");

        let plan = plan_bai_region_chunks(
            &regions,
            &index(),
            Path::new("input.bam.bai"),
            IndexKind::Bai,
            false,
        )
        .expect("plan should build");

        assert_eq!(plan.total_candidate_chunks, 0);
        assert_eq!(plan.total_coalesced_chunks, 0);
        assert!(plan.reference_plans[0].coalesced_chunks.is_empty());
    }

    #[test]
    fn rejects_unsupported_stale_incompatible_and_impossible_indexes() {
        let regions = normalize_region_strings(
            &["chr1:1-10".to_string()],
            &references(),
            Path::new("input.bam"),
        )
        .expect("region should normalize");

        let unsupported = plan_bai_region_chunks(
            &regions,
            &index(),
            Path::new("input.bam.csi"),
            IndexKind::Csi,
            false,
        )
        .expect_err("CSI should be unsupported");
        assert!(matches!(unsupported, AppError::UnsupportedIndex { .. }));

        let stale = plan_bai_region_chunks(
            &regions,
            &index(),
            Path::new("input.bam.bai"),
            IndexKind::Bai,
            true,
        )
        .expect_err("stale index should fail");
        assert!(matches!(stale, AppError::MissingIndex { .. }));

        let incompatible = plan_bai_region_chunks(
            &regions,
            &BaiIndex {
                references: Vec::new(),
                unplaced_unmapped_reads: 0,
            },
            Path::new("input.bam.bai"),
            IndexKind::Bai,
            false,
        )
        .expect_err("reference mismatch should fail");
        assert!(matches!(incompatible, AppError::InvalidIndex { .. }));

        let mut impossible = index();
        impossible.references[0]
            .bins
            .insert(4_681, vec![chunk(200, 100)]);
        let impossible_error = plan_bai_region_chunks(
            &regions,
            &impossible,
            Path::new("input.bam.bai"),
            IndexKind::Bai,
            false,
        )
        .expect_err("impossible chunk should fail");
        assert!(matches!(impossible_error, AppError::InvalidIndex { .. }));
    }
}
