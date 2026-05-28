use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{bam::header::ReferenceRecord, error::AppError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RegionInputKind {
    #[serde(rename = "whole_reference")]
    WholeReference,
    #[serde(rename = "explicit_interval")]
    ExplicitInterval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedRegion {
    pub original: String,
    pub reference_name: String,
    pub reference_index: usize,
    pub reference_length: u32,
    pub start_0_based: u32,
    pub end_0_based_exclusive: u32,
    pub start_1_based_inclusive: u32,
    pub end_1_based_inclusive: u32,
    pub input_kind: RegionInputKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedRegionSet {
    pub regions: Vec<NormalizedRegion>,
    pub coordinate_base: &'static str,
    pub interval_semantics: &'static str,
    pub duplicate_policy: &'static str,
}

#[derive(Debug)]
struct ReferenceLookup<'a> {
    by_name: BTreeMap<&'a str, Vec<&'a ReferenceRecord>>,
}

impl<'a> ReferenceLookup<'a> {
    fn new(references: &'a [ReferenceRecord]) -> Self {
        let mut by_name: BTreeMap<&'a str, Vec<&'a ReferenceRecord>> = BTreeMap::new();
        for reference in references {
            by_name.entry(&reference.name).or_default().push(reference);
        }
        Self { by_name }
    }

    fn resolve_name(
        &self,
        name: &str,
        original: &str,
        path: &Path,
    ) -> Result<&ReferenceRecord, AppError> {
        match self.by_name.get(name).map(Vec::as_slice) {
            Some([reference]) => Ok(*reference),
            Some(_) => Err(invalid_region(
                path,
                format!(
                    "Region '{original}' resolves to duplicate reference name '{name}' in the BAM header dictionary."
                ),
            )),
            None => Err(invalid_region(
                path,
                format!("Region '{original}' references unknown reference '{name}'."),
            )),
        }
    }

    fn interval_candidates<'b>(
        &self,
        original: &'b str,
    ) -> Vec<(&'b str, &'b str, &'a ReferenceRecord)> {
        original
            .match_indices(':')
            .filter_map(|(colon_index, _)| {
                let reference_name = &original[..colon_index];
                let coordinates = &original[(colon_index + 1)..];
                match self.by_name.get(reference_name).map(Vec::as_slice) {
                    Some([reference]) if looks_like_closed_interval(coordinates) => {
                        Some((reference_name, coordinates, *reference))
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

pub fn normalize_region_strings(
    regions: &[String],
    references: &[ReferenceRecord],
    path: &Path,
) -> Result<NormalizedRegionSet, AppError> {
    if regions.is_empty() {
        return Err(invalid_region(
            path,
            "At least one region string is required.".to_string(),
        ));
    }

    let lookup = ReferenceLookup::new(references);
    let mut normalized = Vec::with_capacity(regions.len());
    for region in regions {
        normalized.push(normalize_region_string_with_lookup(region, &lookup, path)?);
    }

    Ok(NormalizedRegionSet {
        regions: normalized,
        coordinate_base: "input_1_based_closed_output_0_based_half_open",
        interval_semantics: "start and end are inclusive in input; normalized end is exclusive",
        duplicate_policy: "preserve_request_order_without_merging_or_deduplication",
    })
}

pub fn normalize_region_string(
    region: &str,
    references: &[ReferenceRecord],
    path: &Path,
) -> Result<NormalizedRegion, AppError> {
    let lookup = ReferenceLookup::new(references);
    normalize_region_string_with_lookup(region, &lookup, path)
}

pub fn reject_region_file_request(path: &Path) -> Result<(), AppError> {
    Err(AppError::Unimplemented {
        path: path.to_path_buf(),
        detail: "Region-file parsing is specified for M11.2 but is not public CLI behavior until select_region is implemented; provide ordered --region strings instead.".to_string(),
    })
}

fn normalize_region_string_with_lookup(
    region: &str,
    lookup: &ReferenceLookup<'_>,
    path: &Path,
) -> Result<NormalizedRegion, AppError> {
    let original = region.trim();
    if original.is_empty() {
        return Err(invalid_region(path, "Region string was empty.".to_string()));
    }
    if original != region {
        return Err(invalid_region(
            path,
            format!("Region '{region}' contains leading or trailing whitespace."),
        ));
    }

    if !original.contains(':') {
        let reference = lookup.resolve_name(original, original, path)?;
        return whole_reference_region(original, reference, path);
    }

    let candidates = lookup.interval_candidates(original);
    if lookup.by_name.contains_key(original) && !candidates.is_empty() {
        return Err(invalid_region(
            path,
            format!(
                "Region '{original}' is ambiguous because it is both an exact reference name and a valid reference:start-end interval."
            ),
        ));
    }

    match candidates.as_slice() {
        [(reference_name, coordinates, reference)] => {
            explicit_interval_region(original, reference_name, coordinates, reference, path)
        }
        [] => {
            if let Ok(reference) = lookup.resolve_name(original, original, path) {
                return whole_reference_region(original, reference, path);
            }
            Err(invalid_region(
                path,
                format!(
                    "Region '{original}' must match reference or reference:start-end with a known, unambiguous reference name."
                ),
            ))
        }
        _ => {
            let names = candidates
                .iter()
                .map(|(name, _, _)| (*name).to_string())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ");
            Err(invalid_region(
                path,
                format!(
                    "Region '{original}' is ambiguous because multiple reference-name splits match: {names}."
                ),
            ))
        }
    }
}

fn whole_reference_region(
    original: &str,
    reference: &ReferenceRecord,
    path: &Path,
) -> Result<NormalizedRegion, AppError> {
    if reference.length == 0 {
        return Err(invalid_region(
            path,
            format!(
                "Region '{original}' targets zero-length reference '{}'.",
                reference.name
            ),
        ));
    }

    Ok(NormalizedRegion {
        original: original.to_string(),
        reference_name: reference.name.clone(),
        reference_index: reference.index,
        reference_length: reference.length,
        start_0_based: 0,
        end_0_based_exclusive: reference.length,
        start_1_based_inclusive: 1,
        end_1_based_inclusive: reference.length,
        input_kind: RegionInputKind::WholeReference,
    })
}

fn explicit_interval_region(
    original: &str,
    reference_name: &str,
    coordinates: &str,
    reference: &ReferenceRecord,
    path: &Path,
) -> Result<NormalizedRegion, AppError> {
    let (start_text, end_text) = coordinates.split_once('-').ok_or_else(|| {
        invalid_region(
            path,
            format!("Region '{original}' is missing an interval end coordinate."),
        )
    })?;
    let start = parse_coordinate(start_text, original, "start", path)?;
    let end = parse_coordinate(end_text, original, "end", path)?;

    if start == 0 || end == 0 {
        return Err(invalid_region(
            path,
            format!("Region '{original}' uses 1-based coordinates; coordinates must be >= 1."),
        ));
    }
    if end < start {
        return Err(invalid_region(
            path,
            format!(
                "Region '{original}' has reversed coordinates: end {end} is before start {start}."
            ),
        ));
    }
    if end > reference.length {
        return Err(invalid_region(
            path,
            format!(
                "Region '{original}' ends at {end}, beyond reference '{reference_name}' length {}.",
                reference.length
            ),
        ));
    }

    Ok(NormalizedRegion {
        original: original.to_string(),
        reference_name: reference.name.clone(),
        reference_index: reference.index,
        reference_length: reference.length,
        start_0_based: start - 1,
        end_0_based_exclusive: end,
        start_1_based_inclusive: start,
        end_1_based_inclusive: end,
        input_kind: RegionInputKind::ExplicitInterval,
    })
}

fn looks_like_closed_interval(value: &str) -> bool {
    let Some((start, end)) = value.split_once('-') else {
        return false;
    };
    !start.is_empty()
        && !end.is_empty()
        && start.bytes().all(|byte| byte.is_ascii_digit())
        && end.bytes().all(|byte| byte.is_ascii_digit())
}

fn parse_coordinate(
    value: &str,
    original: &str,
    label: &str,
    path: &Path,
) -> Result<u32, AppError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_region(
            path,
            format!("Region '{original}' has a non-numeric {label} coordinate '{value}'."),
        ));
    }

    value.parse::<u32>().map_err(|_| {
        invalid_region(
            path,
            format!("Region '{original}' {label} coordinate '{value}' exceeds the u32 limit."),
        )
    })
}

fn invalid_region(path: &Path, detail: String) -> AppError {
    AppError::InvalidRegion {
        path: PathBuf::from(path),
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RegionInputKind, normalize_region_string, normalize_region_strings,
        reject_region_file_request,
    };
    use crate::{
        bam::header::{ReferenceHeaderFields, ReferenceRecord},
        error::AppError,
    };
    use std::path::Path;

    fn references() -> Vec<ReferenceRecord> {
        vec![
            reference("chr1", 100, 0),
            reference("chr2", 200, 1),
            reference("chr:with:colon", 50, 2),
            reference("chr:with", 75, 3),
            reference("chr1:1-2", 80, 4),
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

    #[test]
    fn normalizes_one_based_closed_interval_to_zero_based_half_open() {
        let region = normalize_region_string("chr1:2-10", &references(), Path::new("input.bam"))
            .expect("region should normalize");

        assert_eq!(region.reference_name, "chr1");
        assert_eq!(region.reference_index, 0);
        assert_eq!(region.start_1_based_inclusive, 2);
        assert_eq!(region.end_1_based_inclusive, 10);
        assert_eq!(region.start_0_based, 1);
        assert_eq!(region.end_0_based_exclusive, 10);
        assert_eq!(region.input_kind, RegionInputKind::ExplicitInterval);
    }

    #[test]
    fn normalizes_whole_reference_request() {
        let region = normalize_region_string("chr2", &references(), Path::new("input.bam"))
            .expect("whole reference should normalize");

        assert_eq!(region.reference_name, "chr2");
        assert_eq!(region.start_0_based, 0);
        assert_eq!(region.end_0_based_exclusive, 200);
        assert_eq!(region.start_1_based_inclusive, 1);
        assert_eq!(region.end_1_based_inclusive, 200);
        assert_eq!(region.input_kind, RegionInputKind::WholeReference);
    }

    #[test]
    fn preserves_multiple_regions_without_merging_or_deduplicating() {
        let regions = vec![
            "chr1:1-5".to_string(),
            "chr1:1-5".to_string(),
            "chr2".to_string(),
        ];
        let set = normalize_region_strings(&regions, &references(), Path::new("input.bam"))
            .expect("region set should normalize");

        assert_eq!(set.regions.len(), 3);
        assert_eq!(
            set.duplicate_policy,
            "preserve_request_order_without_merging_or_deduplication"
        );
        assert_eq!(set.regions[0].original, "chr1:1-5");
        assert_eq!(set.regions[1].original, "chr1:1-5");
        assert_eq!(set.regions[2].original, "chr2");
    }

    #[test]
    fn resolves_reference_names_that_contain_colons() {
        let region =
            normalize_region_string("chr:with:colon:5-6", &references(), Path::new("input.bam"))
                .expect("colon-containing reference should normalize");

        assert_eq!(region.reference_name, "chr:with:colon");
        assert_eq!(region.start_0_based, 4);
        assert_eq!(region.end_0_based_exclusive, 6);
    }

    #[test]
    fn rejects_ambiguous_reference_name_splits() {
        let error = normalize_region_string("chr1:1-2", &references(), Path::new("input.bam"))
            .expect_err("ambiguous region should fail");

        assert!(matches!(error, AppError::InvalidRegion { .. }));
        assert!(
            error
                .to_json_error()
                .detail
                .as_deref()
                .unwrap_or_default()
                .contains("ambiguous")
        );
    }

    #[test]
    fn rejects_unknown_reference() {
        let error = normalize_region_string("chr9:1-2", &references(), Path::new("input.bam"))
            .expect_err("unknown reference should fail");

        assert!(matches!(error, AppError::InvalidRegion { .. }));
        assert_eq!(error.to_json_error().code, "invalid_region");
    }

    #[test]
    fn rejects_empty_reversed_zero_and_out_of_range_intervals() {
        for invalid in [
            "",
            "chr1:10-2",
            "chr1:0-2",
            "chr1:1-101",
            "chr1:1-",
            "chr1:a-2",
        ] {
            assert!(
                normalize_region_string(invalid, &references(), Path::new("input.bam")).is_err(),
                "{invalid} should fail"
            );
        }
    }

    #[test]
    fn rejects_duplicate_reference_names_as_ambiguous() {
        let references = vec![reference("chr1", 100, 0), reference("chr1", 100, 1)];
        let error = normalize_region_string("chr1", &references, Path::new("input.bam"))
            .expect_err("duplicate reference names should fail");

        assert!(matches!(error, AppError::InvalidRegion { .. }));
    }

    #[test]
    fn rejects_region_files_as_specified_but_not_yet_public() {
        let error = reject_region_file_request(Path::new("regions.bed"))
            .expect_err("region files should be deferred");

        assert!(matches!(error, AppError::Unimplemented { .. }));
        assert!(
            error
                .to_json_error()
                .detail
                .as_deref()
                .unwrap_or_default()
                .contains("specified for M11.2")
        );
    }
}
