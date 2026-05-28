use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    bam::{
        header::{HeaderPayload, serialize_bam_header_payload},
        index::{
            IndexKind, IndexResolution, bam_newer_than_index, parse_bai_index,
            resolve_index_for_bam,
        },
        record::BamRecordView,
        region::{NormalizedRegion, NormalizedRegionSet, normalize_region_strings},
        region_plan::plan_bai_region_chunks,
        region_traversal::{RegionMatchedRecord, traverse_planned_region_chunks},
        scan::{BamRecordVirtualOffsets, BamScanner},
        write::BgzfWriter,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    output_safety::{finalize_completed_output, remove_stale_temp},
};

#[derive(Debug)]
pub struct SelectRegionRequest {
    pub bam: PathBuf,
    pub out: PathBuf,
    pub regions: Vec<String>,
    pub dry_run: bool,
    pub force: bool,
    pub prefer_index: bool,
}

#[derive(Debug, Serialize)]
pub struct SelectRegionPayload {
    pub format: &'static str,
    pub dry_run: bool,
    pub input: String,
    pub output: SelectRegionOutput,
    pub region_scope: SelectRegionScope,
    pub execution: SelectRegionExecution,
    pub header: SelectRegionHeaderPolicy,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SelectRegionOutput {
    pub path: String,
    pub output_format: &'static str,
    pub compression: &'static str,
    pub output_created: bool,
    pub records_written: u64,
    pub duplicate_emission_policy: &'static str,
    pub output_ordering_policy: &'static str,
    pub index_invalidation: SelectRegionIndexInvalidation,
}

#[derive(Debug, Serialize)]
pub struct SelectRegionIndexInvalidation {
    pub adjacent_index_paths: Vec<String>,
    pub preexisting_index_paths: Vec<String>,
    pub removed_index_paths: Vec<String>,
    pub invalidation_action: &'static str,
    pub output_index_created: bool,
    pub regeneration: String,
}

#[derive(Debug, Serialize)]
pub struct SelectRegionScope {
    pub source: &'static str,
    pub requested_region_count: usize,
    pub normalized_region_count: usize,
    pub duplicate_region_request_count: usize,
    pub regions: Vec<NormalizedRegion>,
}

#[derive(Debug, Serialize)]
pub struct SelectRegionExecution {
    pub mode: SelectRegionExecutionMode,
    pub index_path: Option<String>,
    pub chunks_traversed: usize,
    pub raw_records_seen: usize,
    pub records_examined: usize,
    pub selected_unique_record_count: usize,
    pub duplicate_physical_records_suppressed: usize,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectRegionExecutionMode {
    Indexed,
    ScanFallback,
}

#[derive(Debug, Serialize)]
pub struct SelectRegionHeaderPolicy {
    pub reference_dictionary_preserved: bool,
    pub references_retained: usize,
    pub provenance_pg_appended: bool,
    pub input_sort_order: Option<String>,
    pub input_sub_sort_order: Option<String>,
    pub output_sort_order: Option<String>,
    pub output_sub_sort_order: Option<String>,
    pub sort_metadata_note: String,
}

pub fn run(request: SelectRegionRequest) -> Result<SelectRegionPayload, AppError> {
    validate_output_target(&request)?;
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
            detail: "Input did not present a BGZF-compatible BAM container.".to_string(),
        });
    }

    let scanner = BamScanner::open(&request.bam)?;
    let header = scanner.header().clone();
    let regions =
        normalize_region_strings(&request.regions, &header.header.references, &request.bam)?;
    let duplicate_region_request_count = duplicate_region_request_count(&request.regions);
    let selection = collect_selected_records(&request, &regions, header.header.references.len())?;
    let rewritten_header_text = rewrite_selected_output_header(&header);
    let header_payload = serialize_bam_header_payload(
        &request.out,
        &rewritten_header_text,
        &header.header.references,
    )?;
    let index_invalidation =
        prepare_output_index_invalidation(&request.out, request.force, request.dry_run)?;

    if !request.dry_run {
        write_selected_bam(
            &request.out,
            request.force,
            &header_payload,
            &selection.records,
        )?;
    }

    let header_policy = SelectRegionHeaderPolicy {
        reference_dictionary_preserved: true,
        references_retained: header.header.references.len(),
        provenance_pg_appended: true,
        input_sort_order: header.header.hd.sort_order.clone(),
        input_sub_sort_order: header.header.hd.sub_sort_order.clone(),
        output_sort_order: header.header.hd.sort_order.as_ref().map(|_| "unknown".to_string()),
        output_sub_sort_order: None,
        sort_metadata_note: "Selected-region output downgrades sort metadata because source-order region selection is not guaranteed to preserve whole-file coordinate or queryname order.".to_string(),
    };

    let mut notes = vec![
        "Selected BAM output preserves raw alignment record bytes for retained records.".to_string(),
        "Each physical source BAM record is emitted at most once in source virtual-offset order.".to_string(),
        "select_region does not create an output BAM index; run bamana index on the selected BAM when an index is required.".to_string(),
        "Adjacent output index sidecars are rejected unless --force is supplied; forced applied runs remove those sidecars before writing selected output.".to_string(),
    ];
    if selection.mode == SelectRegionExecutionMode::ScanFallback {
        notes.push("No usable BAI index was used; native scan fallback selected records in encounter order.".to_string());
    }
    if request.dry_run {
        notes.push("Dry-run mode planned selection without writing BAM output.".to_string());
    }

    Ok(SelectRegionPayload {
        format: "BAM",
        dry_run: request.dry_run,
        input: request.bam.to_string_lossy().into_owned(),
        output: SelectRegionOutput {
            path: request.out.to_string_lossy().into_owned(),
            output_format: "bam",
            compression: "bgzf",
            output_created: !request.dry_run,
            records_written: if request.dry_run {
                0
            } else {
                selection.records.len() as u64
            },
            duplicate_emission_policy: "emit_once_per_source_record",
            output_ordering_policy: "source_virtual_offset_order",
            index_invalidation,
        },
        region_scope: SelectRegionScope {
            source: "cli_regions",
            requested_region_count: request.regions.len(),
            normalized_region_count: regions.regions.len(),
            duplicate_region_request_count,
            regions: regions.regions,
        },
        execution: SelectRegionExecution {
            mode: selection.mode,
            index_path: selection.index_path,
            chunks_traversed: selection.chunks_traversed,
            raw_records_seen: selection.raw_records_seen,
            records_examined: selection.raw_records_seen,
            selected_unique_record_count: selection.records.len(),
            duplicate_physical_records_suppressed: selection.duplicate_records_suppressed,
            fallback_reason: selection.fallback_reason,
        },
        header: header_policy,
        notes,
    })
}

struct SelectionResult {
    records: Vec<RegionMatchedRecord>,
    mode: SelectRegionExecutionMode,
    index_path: Option<String>,
    chunks_traversed: usize,
    raw_records_seen: usize,
    duplicate_records_suppressed: usize,
    fallback_reason: Option<String>,
}

fn collect_selected_records(
    request: &SelectRegionRequest,
    regions: &NormalizedRegionSet,
    references_defined: usize,
) -> Result<SelectionResult, AppError> {
    if request.prefer_index {
        if let Some(result) = try_indexed_selection(request, regions, references_defined)? {
            return Ok(result);
        }
    }
    scan_fallback_selection(request, regions)
}

fn try_indexed_selection(
    request: &SelectRegionRequest,
    regions: &NormalizedRegionSet,
    references_defined: usize,
) -> Result<Option<SelectionResult>, AppError> {
    let resolved = match resolve_index_for_bam(&request.bam) {
        IndexResolution::Present(resolved) => resolved,
        IndexResolution::Unsupported(_) | IndexResolution::NotFound => return Ok(None),
    };
    if resolved.kind != IndexKind::Bai
        || bam_newer_than_index(&request.bam, &resolved.path).unwrap_or(true)
    {
        return Ok(None);
    }
    let index = match parse_bai_index(&resolved.path, references_defined) {
        Ok(index) => index,
        Err(_) => return Ok(None),
    };
    let plan = match plan_bai_region_chunks(regions, &index, &resolved.path, resolved.kind, false) {
        Ok(plan) => plan,
        Err(_) => return Ok(None),
    };
    let traversal = traverse_planned_region_chunks(&request.bam, regions, &plan)?;
    Ok(Some(SelectionResult {
        records: traversal.records,
        mode: SelectRegionExecutionMode::Indexed,
        index_path: Some(resolved.path.to_string_lossy().into_owned()),
        chunks_traversed: traversal.chunks_traversed,
        raw_records_seen: traversal.raw_records_seen,
        duplicate_records_suppressed: traversal.duplicate_records_suppressed,
        fallback_reason: None,
    }))
}

fn scan_fallback_selection(
    request: &SelectRegionRequest,
    regions: &NormalizedRegionSet,
) -> Result<SelectionResult, AppError> {
    let mut scanner = BamScanner::open(&request.bam)?;
    let grouped = group_regions_by_reference(regions);
    let mut records = BTreeMap::<(u64, u64), RegionMatchedRecord>::new();
    let mut raw_records_seen = 0_usize;
    let mut duplicate_records_suppressed = 0_usize;

    while let Some(raw) = scanner.next_raw_record_with_virtual_offsets()? {
        raw_records_seen += 1;
        let view =
            BamRecordView::parse(&raw.raw_record).map_err(|error| AppError::InvalidRecord {
                path: request.bam.clone(),
                detail: error.detail().to_string(),
            })?;
        let Some((reference_index, start, end)) = mapped_record_interval(&view, &request.bam)?
        else {
            continue;
        };
        let Some(reference_regions) = grouped.get(&reference_index) else {
            continue;
        };
        let matched_regions = reference_regions
            .iter()
            .filter(|region| start < region.end_0_based_exclusive && end > region.start_0_based)
            .map(|region| region.original.clone())
            .collect::<Vec<_>>();
        if matched_regions.is_empty() {
            continue;
        }
        let key = virtual_offset_key(raw.virtual_offsets);
        if let Some(existing) = records.get_mut(&key) {
            duplicate_records_suppressed += 1;
            merge_region_matches(&mut existing.matched_regions, matched_regions);
        } else {
            records.insert(
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

    Ok(SelectionResult {
        records: records.into_values().collect(),
        mode: SelectRegionExecutionMode::ScanFallback,
        index_path: None,
        chunks_traversed: 0,
        raw_records_seen,
        duplicate_records_suppressed,
        fallback_reason: Some("no_usable_bai_index".to_string()),
    })
}

fn write_selected_bam(
    output: &Path,
    force: bool,
    header_payload: &[u8],
    records: &[RegionMatchedRecord],
) -> Result<(), AppError> {
    if output.exists() && !force {
        return Err(AppError::OutputExists {
            path: output.to_path_buf(),
        });
    }
    let temp_path = temporary_output_path(output);
    remove_stale_temp(&temp_path);
    let write_result = (|| -> Result<(), AppError> {
        let mut writer = BgzfWriter::create(&temp_path)?;
        writer.write_all(header_payload)?;
        for record in records {
            writer.write_all(&record.raw_record)?;
        }
        writer.finish()?;
        Ok(())
    })();
    if let Err(error) = write_result {
        remove_stale_temp(&temp_path);
        return Err(error);
    }
    finalize_completed_output(&temp_path, output, force)
}

fn validate_output_target(request: &SelectRegionRequest) -> Result<(), AppError> {
    if request.out.as_os_str() == "-" {
        return Err(AppError::Unimplemented {
            path: request.out.clone(),
            detail: "select_region --out - requires binary-stdout response routing; file output is implemented in M11.6 and stdout BAM output remains deferred.".to_string(),
        });
    }
    if request.out == request.bam || existing_paths_are_same_file(&request.bam, &request.out) {
        return Err(AppError::UnsupportedInputForCommand {
            path: request.out.clone(),
            detail: "select_region refuses same-path input/output rewrites; write to a distinct BAM path."
                .to_string(),
        });
    }
    Ok(())
}

fn prepare_output_index_invalidation(
    output: &Path,
    force: bool,
    dry_run: bool,
) -> Result<SelectRegionIndexInvalidation, AppError> {
    let adjacent = output_index_candidate_paths(output);
    let existing = adjacent
        .iter()
        .filter(|path| path.is_file())
        .cloned()
        .collect::<Vec<_>>();
    let regeneration = format!("bamana index --input {}", output.to_string_lossy());

    if dry_run {
        return Ok(SelectRegionIndexInvalidation {
            adjacent_index_paths: stringify_paths(adjacent),
            preexisting_index_paths: stringify_paths(existing.clone()),
            removed_index_paths: Vec::new(),
            invalidation_action: if existing.is_empty() {
                "would_leave_no_output_index"
            } else {
                "would_remove_existing_index_sidecars_on_apply"
            },
            output_index_created: false,
            regeneration,
        });
    }

    if let Some(first_existing) = existing.first() {
        if !force {
            return Err(AppError::OutputExists {
                path: first_existing.clone(),
            });
        }
    }

    let mut removed = Vec::new();
    for path in &existing {
        fs::remove_file(path).map_err(|error| AppError::WriteError {
            path: path.clone(),
            message: format!("failed to remove stale output index sidecar: {error}"),
        })?;
        removed.push(path.clone());
    }

    let invalidation_action = if removed.is_empty() {
        "no_existing_index_sidecars"
    } else {
        "removed_existing_index_sidecars"
    };

    Ok(SelectRegionIndexInvalidation {
        adjacent_index_paths: stringify_paths(adjacent),
        preexisting_index_paths: stringify_paths(existing),
        removed_index_paths: stringify_paths(removed),
        invalidation_action,
        output_index_created: false,
        regeneration,
    })
}

fn output_index_candidate_paths(output: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    push_unique_path(
        &mut paths,
        PathBuf::from(format!("{}.bai", output.to_string_lossy())),
    );
    let mut plain_bai = output.to_path_buf();
    plain_bai.set_extension("bai");
    push_unique_path(&mut paths, plain_bai);
    push_unique_path(
        &mut paths,
        PathBuf::from(format!("{}.csi", output.to_string_lossy())),
    );
    let mut plain_csi = output.to_path_buf();
    plain_csi.set_extension("csi");
    push_unique_path(&mut paths, plain_csi);
    paths
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn stringify_paths(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

fn existing_paths_are_same_file(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn rewrite_selected_output_header(header: &HeaderPayload) -> String {
    let mut lines = Vec::new();
    let mut updated_hd = false;
    let pg_id = next_program_id(&header.header.raw_header_text);
    let previous_program = terminal_program_id(&header.header.raw_header_text);

    for line in header.header.raw_header_text.lines() {
        if line.starts_with("@HD") && !updated_hd {
            lines.push(rewrite_hd_unknown(line));
            updated_hd = true;
        } else if !line.is_empty() {
            lines.push(line.to_string());
        }
    }

    let mut pg = format!("@PG\tID:{pg_id}\tPN:bamana");
    pg.push_str("\tVN:");
    pg.push_str(env!("CARGO_PKG_VERSION"));
    if let Some(previous_program) = previous_program {
        pg.push_str("\tPP:");
        pg.push_str(&previous_program);
    }
    pg.push_str("\tCL:bamana select_region");
    lines.push(pg);

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn rewrite_hd_unknown(line: &str) -> String {
    let mut fields = Vec::new();
    for field in line.split('\t').skip(1) {
        let Some((key, _)) = field.split_once(':') else {
            continue;
        };
        if matches!(key, "SO" | "SS") {
            continue;
        }
        fields.push(field.to_string());
    }
    fields.push("SO:unknown".to_string());
    format!("@HD\t{}", fields.join("\t"))
}

fn next_program_id(raw_header_text: &str) -> String {
    let existing = program_ids(raw_header_text);
    let base = "bamana_select_region";
    if !existing.contains(base) {
        return base.to_string();
    }
    for index in 1.. {
        let candidate = format!("{base}_{index}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must find a program id")
}

fn terminal_program_id(raw_header_text: &str) -> Option<String> {
    let ids = program_ids(raw_header_text);
    let previous = previous_program_ids(raw_header_text);
    let terminal = ids
        .into_iter()
        .filter(|id| !previous.contains(id.as_str()))
        .collect::<Vec<_>>();
    match terminal.as_slice() {
        [id] => Some((*id).to_string()),
        _ => None,
    }
}

fn program_ids(raw_header_text: &str) -> BTreeSet<String> {
    raw_header_text
        .lines()
        .filter(|line| line.starts_with("@PG"))
        .filter_map(|line| header_field(line, "ID"))
        .collect()
}

fn previous_program_ids(raw_header_text: &str) -> BTreeSet<String> {
    raw_header_text
        .lines()
        .filter(|line| line.starts_with("@PG"))
        .filter_map(|line| header_field(line, "PP"))
        .collect()
}

fn header_field(line: &str, tag: &str) -> Option<String> {
    line.split('\t').skip(1).find_map(|field| {
        let (key, value) = field.split_once(':')?;
        (key == tag).then(|| value.to_string())
    })
}

fn duplicate_region_request_count(regions: &[String]) -> usize {
    let mut seen = BTreeSet::new();
    regions
        .iter()
        .filter(|region| !seen.insert(region.as_str()))
        .count()
}

fn group_regions_by_reference(
    regions: &NormalizedRegionSet,
) -> BTreeMap<usize, Vec<&NormalizedRegion>> {
    let mut grouped = BTreeMap::new();
    for region in &regions.regions {
        grouped
            .entry(region.reference_index)
            .or_insert_with(Vec::new)
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
    Ok(Some((
        reference_index,
        start,
        start
            .checked_add(span)
            .ok_or_else(|| AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: "BAM record reference span overflowed u32.".to_string(),
            })?,
    )))
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

fn merge_region_matches(existing: &mut Vec<String>, additional: Vec<String>) {
    let mut seen = existing.iter().cloned().collect::<BTreeSet<_>>();
    for region in additional {
        if seen.insert(region.clone()) {
            existing.push(region);
        }
    }
}

fn virtual_offset_key(offsets: BamRecordVirtualOffsets) -> (u64, u64) {
    (offsets.start.packed(), offsets.end.packed())
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-select-region-output");
    output.with_file_name(format!(
        ".{file_name}.bamana-select-region-{}.tmp",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{
        bam::{
            header::parse_bam_header,
            index::{build_bai_index_from_bam, write_bai_index},
            scan::BamScanner,
        },
        commands::select_region::{SelectRegionExecutionMode, SelectRegionRequest, run},
        error::AppError,
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    fn region_values(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn indexed_fixture(name: &str) -> (PathBuf, PathBuf) {
        let records = vec![
            build_light_record(0, 5, "read1", 0),
            build_light_record(0, 8, "read2", 0),
            build_light_record(0, 50, "read3", 0),
        ];
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\tSS:coordinate:minhash\n@SQ\tSN:chr1\tLN:100\n@PG\tID:aligner\tPN:aligner\n",
            &[("chr1", 100)],
            &records,
        );
        let bam = write_temp_file(name, "bam", &bytes);
        let bai = PathBuf::from(format!("{}.bai", bam.to_string_lossy()));
        let index = build_bai_index_from_bam(&bam).expect("index should build");
        write_bai_index(&bai, &index).expect("index should write");
        (bam, bai)
    }

    #[test]
    fn indexed_selection_writes_matching_raw_records_once() {
        let (bam, bai) = indexed_fixture("select-region-indexed");
        let out = bam.with_extension("selected.bam");

        let payload = run(SelectRegionRequest {
            bam: bam.clone(),
            out: out.clone(),
            regions: region_values(&["chr1:6-9", "chr1:8-9"]),
            dry_run: false,
            force: false,
            prefer_index: true,
        })
        .expect("selection should succeed");

        assert_eq!(payload.execution.mode, SelectRegionExecutionMode::Indexed);
        assert_eq!(payload.output.records_written, 2);
        assert_eq!(payload.execution.selected_unique_record_count, 2);
        let header = parse_bam_header(&out).expect("output header should parse");
        assert_eq!(header.header.references.len(), 1);
        assert_eq!(header.header.hd.sort_order.as_deref(), Some("unknown"));
        assert_eq!(header.header.hd.sub_sort_order, None);
        assert!(
            header
                .header
                .raw_header_text
                .contains("@PG\tID:bamana_select_region")
        );
        let mut scanner = BamScanner::open(&out).expect("output should scan");
        let mut names = Vec::new();
        while let Some(record) = scanner.next_record().expect("record should scan") {
            names.push(record.read_name().to_string());
        }
        assert_eq!(names, vec!["read1", "read2"]);

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");
        fs::remove_file(out).expect("out should remove");
    }

    #[test]
    fn dry_run_reports_without_writing_output() {
        let (bam, bai) = indexed_fixture("select-region-dry-run");
        let out = bam.with_extension("dry.selected.bam");

        let payload = run(SelectRegionRequest {
            bam: bam.clone(),
            out: out.clone(),
            regions: region_values(&["chr1:6-9"]),
            dry_run: true,
            force: false,
            prefer_index: true,
        })
        .expect("dry-run should succeed");

        assert!(payload.dry_run);
        assert!(!payload.output.output_created);
        assert_eq!(payload.output.records_written, 0);
        assert!(!out.exists());

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");
    }

    #[test]
    fn existing_output_index_sidecar_requires_force() {
        let (bam, bai) = indexed_fixture("select-region-index-collision");
        let out = bam.with_extension("collision.selected.bam");
        let output_index = PathBuf::from(format!("{}.bai", out.to_string_lossy()));
        fs::write(&output_index, b"stale-index").expect("stale index should write");

        let error = run(SelectRegionRequest {
            bam: bam.clone(),
            out: out.clone(),
            regions: region_values(&["chr1:6-9"]),
            dry_run: false,
            force: false,
            prefer_index: true,
        })
        .expect_err("stale adjacent output index should require force");

        assert!(matches!(error, AppError::OutputExists { path } if path == output_index));
        assert!(output_index.exists());
        assert!(!out.exists());

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");
        fs::remove_file(output_index).expect("output index should remove");
    }

    #[test]
    fn force_removes_existing_output_index_sidecar_before_writing() {
        let (bam, bai) = indexed_fixture("select-region-index-force");
        let out = bam.with_extension("force.selected.bam");
        let output_index = PathBuf::from(format!("{}.bai", out.to_string_lossy()));
        fs::write(&output_index, b"stale-index").expect("stale index should write");

        let payload = run(SelectRegionRequest {
            bam: bam.clone(),
            out: out.clone(),
            regions: region_values(&["chr1:6-9"]),
            dry_run: false,
            force: true,
            prefer_index: true,
        })
        .expect("force should remove stale output index and write output");

        assert!(payload.output.output_created);
        assert_eq!(
            payload.output.index_invalidation.invalidation_action,
            "removed_existing_index_sidecars"
        );
        assert_eq!(
            payload.output.index_invalidation.removed_index_paths,
            vec![output_index.to_string_lossy().into_owned()]
        );
        assert!(!payload.output.index_invalidation.output_index_created);
        assert!(!output_index.exists());
        assert!(out.exists());

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");
        fs::remove_file(out).expect("out should remove");
    }

    #[test]
    fn dry_run_reports_index_invalidation_without_removing_sidecar() {
        let (bam, bai) = indexed_fixture("select-region-index-dry-run");
        let out = bam.with_extension("index-dry.selected.bam");
        let output_index = PathBuf::from(format!("{}.bai", out.to_string_lossy()));
        fs::write(&output_index, b"stale-index").expect("stale index should write");

        let payload = run(SelectRegionRequest {
            bam: bam.clone(),
            out: out.clone(),
            regions: region_values(&["chr1:6-9"]),
            dry_run: true,
            force: false,
            prefer_index: true,
        })
        .expect("dry run should report planned index invalidation");

        assert_eq!(
            payload.output.index_invalidation.invalidation_action,
            "would_remove_existing_index_sidecars_on_apply"
        );
        assert_eq!(
            payload.output.index_invalidation.removed_index_paths.len(),
            0
        );
        assert!(output_index.exists());
        assert!(!out.exists());

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");
        fs::remove_file(output_index).expect("output index should remove");
    }

    #[test]
    fn same_path_input_output_is_rejected() {
        let (bam, bai) = indexed_fixture("select-region-same-path");

        let error = run(SelectRegionRequest {
            bam: bam.clone(),
            out: bam.clone(),
            regions: region_values(&["chr1:6-9"]),
            dry_run: false,
            force: true,
            prefer_index: true,
        })
        .expect_err("same-path rewrite should be rejected");

        assert!(matches!(error, AppError::UnsupportedInputForCommand { .. }));

        fs::remove_file(bam).expect("bam should remove");
        fs::remove_file(bai).expect("bai should remove");
    }
}
