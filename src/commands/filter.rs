use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    bam::{
        header::serialize_bam_header_payload, record::BamRecordView, scan::BamScanner,
        write::BgzfWriter,
    },
    cli::FilterComplexityNonCanonicalPolicy,
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    output_safety::{finalize_completed_output, remove_stale_temp},
};

#[derive(Debug)]
pub struct FilterRequest {
    pub bam: PathBuf,
    pub out: PathBuf,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub min_mean_quality: Option<f64>,
    pub max_mean_quality: Option<f64>,
    pub min_complexity: Option<f64>,
    pub max_complexity: Option<f64>,
    pub complexity_k_min: usize,
    pub complexity_k_max: usize,
    pub complexity_noncanonical: FilterComplexityNonCanonicalPolicy,
    pub mapped_only: bool,
    pub unmapped_only: bool,
    pub primary_only: bool,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct FilterPayload {
    pub format: &'static str,
    pub dry_run: bool,
    pub input: String,
    pub filters: FilterPolicy,
    pub execution: FilterExecution,
    pub output: FilterOutput,
    pub header: FilterHeaderPolicy,
    pub index: FilterIndexPolicy,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FilterPolicy {
    pub mapped_only: bool,
    pub unmapped_only: bool,
    pub primary_only: bool,
    pub length: LengthFilterPolicy,
    pub mean_quality: MeanQualityFilterPolicy,
    pub complexity: ComplexityFilterPolicy,
}

#[derive(Debug, Serialize)]
pub struct LengthFilterPolicy {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct MeanQualityFilterPolicy {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub missing_quality_policy: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ComplexityFilterPolicy {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub k_min: Option<usize>,
    pub k_max: Option<usize>,
    pub method: &'static str,
    pub noncanonical_policy: FilterComplexityNonCanonicalPolicy,
}

#[derive(Debug, Serialize)]
pub struct FilterExecution {
    pub records_examined: u64,
    pub records_retained: u64,
    pub records_removed: u64,
    pub order_preserved: bool,
    pub quality_records_evaluated: u64,
    pub records_removed_missing_quality: u64,
    pub complexity_records_evaluated: u64,
    pub records_removed_noncanonical_complexity: u64,
}

#[derive(Debug, Serialize)]
pub struct FilterOutput {
    pub path: String,
    pub written: bool,
    pub overwritten: bool,
    pub records_written: u64,
}

#[derive(Debug, Serialize)]
pub struct FilterHeaderPolicy {
    pub reference_dictionary_preserved: bool,
    pub references_retained: usize,
    pub raw_header_preserved: bool,
}

#[derive(Debug, Serialize)]
pub struct FilterIndexPolicy {
    pub output_index_created: bool,
    pub adjacent_index_paths: Vec<String>,
    pub preexisting_index_paths: Vec<String>,
    pub removed_index_paths: Vec<String>,
    pub invalidation_action: &'static str,
    pub regeneration: String,
}

#[derive(Default)]
struct FilterCounters {
    records_examined: u64,
    records_retained: u64,
    quality_records_evaluated: u64,
    records_removed_missing_quality: u64,
    complexity_records_evaluated: u64,
    records_removed_noncanonical_complexity: u64,
}

pub fn run(request: FilterRequest) -> Result<FilterPayload, AppError> {
    validate_request(&request)?;
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

    let mut scanner = BamScanner::open(&request.bam)?;
    let header = scanner.header().clone();
    let header_payload = serialize_bam_header_payload(
        &request.out,
        &header.header.raw_header_text,
        &header.header.references,
    )?;
    let output_overwritten = request.out.exists();
    let index_policy = prepare_output_index_policy(&request.out, request.force, request.dry_run)?;

    if request.dry_run {
        let counters = scan_filter_records(&mut scanner, &request, None)?;
        return Ok(payload(
            &request,
            &header,
            counters,
            false,
            output_overwritten,
            index_policy,
        ));
    }

    if output_overwritten && !request.force {
        return Err(AppError::OutputExists {
            path: request.out.clone(),
        });
    }

    let temp_path = temporary_output_path(&request.out);
    remove_stale_temp(&temp_path);
    let write_result = (|| -> Result<FilterCounters, AppError> {
        let mut writer = BgzfWriter::create(&temp_path)?;
        writer.write_all(&header_payload)?;
        let counters = scan_filter_records(&mut scanner, &request, Some(&mut writer))?;
        writer.finish()?;
        Ok(counters)
    })();
    let counters = match write_result {
        Ok(counters) => counters,
        Err(error) => {
            remove_stale_temp(&temp_path);
            return Err(error);
        }
    };
    finalize_completed_output(&temp_path, &request.out, request.force)?;

    Ok(payload(
        &request,
        &header,
        counters,
        true,
        output_overwritten,
        index_policy,
    ))
}

fn scan_filter_records(
    scanner: &mut BamScanner,
    request: &FilterRequest,
    mut writer: Option<&mut BgzfWriter>,
) -> Result<FilterCounters, AppError> {
    let mut counters = FilterCounters::default();
    while let Some(record) = scanner.next_record()? {
        counters.records_examined += 1;
        if record_passes_filters(&record, request, &mut counters)? {
            counters.records_retained += 1;
            if let Some(writer) = writer.as_deref_mut() {
                writer.write_all(record.raw_record())?;
            }
        }
    }
    Ok(counters)
}

fn record_passes_filters(
    record: &BamRecordView<'_>,
    request: &FilterRequest,
    counters: &mut FilterCounters,
) -> Result<bool, AppError> {
    if request.mapped_only && !record_is_mapped(record) {
        return Ok(false);
    }
    if request.unmapped_only && record_is_mapped(record) {
        return Ok(false);
    }
    if request.primary_only && !record.flag_summary().is_primary() {
        return Ok(false);
    }
    if let Some(min_length) = request.min_length
        && record.sequence_len() < min_length
    {
        return Ok(false);
    }
    if let Some(max_length) = request.max_length
        && record.sequence_len() > max_length
    {
        return Ok(false);
    }
    if request.min_mean_quality.is_some() || request.max_mean_quality.is_some() {
        counters.quality_records_evaluated += 1;
        let Some(mean_quality) = mean_quality(record.quality_bytes()) else {
            counters.records_removed_missing_quality += 1;
            return Ok(false);
        };
        if let Some(min_quality) = request.min_mean_quality
            && mean_quality < min_quality
        {
            return Ok(false);
        }
        if let Some(max_quality) = request.max_mean_quality
            && mean_quality > max_quality
        {
            return Ok(false);
        }
    }
    if request.min_complexity.is_some() || request.max_complexity.is_some() {
        counters.complexity_records_evaluated += 1;
        let complexity =
            linguistic_complexity_bam(record.sequence_bytes(), record.sequence_len(), request)
                .map_err(|error| match request.complexity_noncanonical {
                    FilterComplexityNonCanonicalPolicy::Drop => {
                        counters.records_removed_noncanonical_complexity += 1;
                        AppError::Internal {
                            message: error.to_string(),
                        }
                    }
                    FilterComplexityNonCanonicalPolicy::Fail => AppError::InvalidRecord {
                        path: request.bam.clone(),
                        detail: error.to_string(),
                    },
                });
        let complexity = match complexity {
            Ok(value) => value,
            Err(AppError::Internal { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        if let Some(min_complexity) = request.min_complexity
            && complexity < min_complexity
        {
            return Ok(false);
        }
        if let Some(max_complexity) = request.max_complexity
            && complexity > max_complexity
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn record_is_mapped(record: &BamRecordView<'_>) -> bool {
    !record.flag_summary().is_unmapped && record.ref_id() >= 0
}

fn mean_quality(qualities: &[u8]) -> Option<f64> {
    let mut sum = 0_u64;
    let mut count = 0_u64;
    for &quality in qualities {
        if quality != 0xff {
            sum += u64::from(quality);
            count += 1;
        }
    }
    (count > 0).then(|| sum as f64 / count as f64)
}

#[derive(Debug)]
enum ComplexityFailure {
    InvalidKRange,
    UnsupportedSymbol { symbol: char, position: usize },
}

impl std::fmt::Display for ComplexityFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKRange => formatter
                .write_str("Invalid linguistic-complexity k-mer range for this record length."),
            Self::UnsupportedSymbol { symbol, position } => write!(
                formatter,
                "Linguistic complexity requires canonical A/C/G/T bases; encountered '{symbol}' at one-based position {position}."
            ),
        }
    }
}

fn linguistic_complexity_bam(
    sequence_bytes: &[u8],
    sequence_len: usize,
    request: &FilterRequest,
) -> Result<f64, ComplexityFailure> {
    let k_min = request.complexity_k_min;
    let k_max = request.complexity_k_max;
    if k_min == 0 || k_max == 0 || k_min > k_max || k_max > sequence_len {
        return Err(ComplexityFailure::InvalidKRange);
    }

    let mut observed_total = 0_usize;
    let mut possible_total = 0_usize;
    for k in k_min..=k_max {
        observed_total += distinct_canonical_kmers(sequence_bytes, sequence_len, k)?;
        possible_total += possible_distinct_kmers(sequence_len, k);
    }
    Ok(if possible_total == 0 {
        0.0
    } else {
        observed_total as f64 / possible_total as f64
    })
}

fn distinct_canonical_kmers(
    sequence_bytes: &[u8],
    sequence_len: usize,
    k: usize,
) -> Result<usize, ComplexityFailure> {
    let mut kmers = Vec::with_capacity(sequence_len.saturating_sub(k) + 1);
    for start in 0..=sequence_len - k {
        let mut code = 0_u128;
        for offset in 0..k {
            let position = start + offset;
            let base = bam_base_code(sequence_bytes, position)?;
            code = (code << 2) | u128::from(base);
        }
        kmers.push(code);
    }
    kmers.sort_unstable();
    kmers.dedup();
    Ok(kmers.len())
}

fn bam_base_code(sequence_bytes: &[u8], position: usize) -> Result<u8, ComplexityFailure> {
    let packed = sequence_bytes[position / 2];
    let code = if position.is_multiple_of(2) {
        packed >> 4
    } else {
        packed & 0x0f
    };
    match code {
        1 => Ok(0),
        2 => Ok(1),
        4 => Ok(2),
        8 => Ok(3),
        0 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: '=',
            position: position + 1,
        }),
        3 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'M',
            position: position + 1,
        }),
        5 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'R',
            position: position + 1,
        }),
        6 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'S',
            position: position + 1,
        }),
        7 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'V',
            position: position + 1,
        }),
        9 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'W',
            position: position + 1,
        }),
        10 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'Y',
            position: position + 1,
        }),
        11 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'H',
            position: position + 1,
        }),
        12 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'K',
            position: position + 1,
        }),
        13 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'D',
            position: position + 1,
        }),
        14 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'B',
            position: position + 1,
        }),
        15 => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: 'N',
            position: position + 1,
        }),
        other => Err(ComplexityFailure::UnsupportedSymbol {
            symbol: char::from_digit(u32::from(other), 16).unwrap_or('?'),
            position: position + 1,
        }),
    }
}

fn possible_distinct_kmers(length: usize, k: usize) -> usize {
    4_usize
        .checked_pow(k as u32)
        .unwrap_or(usize::MAX)
        .min(length - k + 1)
}

fn validate_request(request: &FilterRequest) -> Result<(), AppError> {
    if request.out == request.bam || existing_paths_are_same_file(&request.bam, &request.out) {
        return Err(AppError::InvalidFilterRequest {
            path: request.out.clone(),
            detail: "filter refuses same-path input/output rewrites; write to a distinct BAM path."
                .to_string(),
        });
    }
    if request.mapped_only && request.unmapped_only {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "--mapped-only and --unmapped-only are mutually exclusive.".to_string(),
        });
    }
    if let (Some(min), Some(max)) = (request.min_length, request.max_length)
        && min > max
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "--min-length cannot exceed --max-length.".to_string(),
        });
    }
    if let (Some(min), Some(max)) = (request.min_mean_quality, request.max_mean_quality)
        && min > max
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "--min-mean-quality cannot exceed --max-mean-quality.".to_string(),
        });
    }
    for (label, value) in [
        ("--min-mean-quality", request.min_mean_quality),
        ("--max-mean-quality", request.max_mean_quality),
        ("--min-complexity", request.min_complexity),
        ("--max-complexity", request.max_complexity),
    ] {
        if let Some(value) = value
            && (!value.is_finite() || value < 0.0)
        {
            return Err(AppError::InvalidFilterRequest {
                path: request.bam.clone(),
                detail: format!("{label} must be a finite non-negative number."),
            });
        }
    }
    if request.min_complexity.is_some_and(|value| value > 1.0)
        || request.max_complexity.is_some_and(|value| value > 1.0)
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "Complexity thresholds must be between 0 and 1.".to_string(),
        });
    }
    if let (Some(min), Some(max)) = (request.min_complexity, request.max_complexity)
        && min > max
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "--min-complexity cannot exceed --max-complexity.".to_string(),
        });
    }
    if (request.min_complexity.is_some() || request.max_complexity.is_some())
        && (request.complexity_k_min == 0
            || request.complexity_k_max == 0
            || request.complexity_k_min > request.complexity_k_max
            || request.complexity_k_max > 64)
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "Complexity k-mer bounds must satisfy 1 <= --complexity-k-min <= --complexity-k-max <= 64.".to_string(),
        });
    }
    Ok(())
}

fn prepare_output_index_policy(
    output: &Path,
    force: bool,
    dry_run: bool,
) -> Result<FilterIndexPolicy, AppError> {
    let adjacent = output_index_candidate_paths(output);
    let existing = adjacent
        .iter()
        .filter(|path| path.is_file())
        .cloned()
        .collect::<Vec<_>>();
    let regeneration = format!("bamana index --input {}", output.to_string_lossy());

    if dry_run {
        return Ok(FilterIndexPolicy {
            output_index_created: false,
            adjacent_index_paths: stringify_paths(adjacent),
            preexisting_index_paths: stringify_paths(existing.clone()),
            removed_index_paths: Vec::new(),
            invalidation_action: if existing.is_empty() {
                "would_leave_no_output_index"
            } else {
                "would_remove_existing_index_sidecars_on_apply"
            },
            regeneration,
        });
    }
    if !existing.is_empty() && !force {
        return Err(AppError::OutputExists {
            path: existing[0].clone(),
        });
    }

    let mut removed = Vec::new();
    for path in &existing {
        fs::remove_file(path).map_err(|error| AppError::WriteError {
            path: path.clone(),
            message: error.to_string(),
        })?;
        removed.push(path.clone());
    }

    let removed_any = !removed.is_empty();
    Ok(FilterIndexPolicy {
        output_index_created: false,
        adjacent_index_paths: stringify_paths(adjacent),
        preexisting_index_paths: stringify_paths(existing),
        removed_index_paths: stringify_paths(removed),
        invalidation_action: if removed_any {
            "removed_existing_index_sidecars"
        } else {
            "left_no_output_index"
        },
        regeneration,
    })
}

fn payload(
    request: &FilterRequest,
    header: &crate::bam::header::HeaderPayload,
    counters: FilterCounters,
    written: bool,
    overwritten: bool,
    index: FilterIndexPolicy,
) -> FilterPayload {
    let records_removed = counters.records_examined - counters.records_retained;
    let complexity_requested = request.min_complexity.is_some() || request.max_complexity.is_some();
    let mut notes = vec![
        "Filtered BAM output preserves raw alignment record bytes for retained records."
            .to_string(),
        "Retained record order follows input encounter order.".to_string(),
        "filter does not create an output BAM index; run bamana index on the filtered BAM when an index is required.".to_string(),
    ];
    if complexity_requested {
        notes.push("Linguistic complexity uses canonical A/C/G/T k-mer diversity with the EMBOSS-RS complex formula and no plot generation.".to_string());
    }
    if request.dry_run {
        notes.push("Dry-run mode counted retained records without writing BAM output.".to_string());
    }

    FilterPayload {
        format: "BAM",
        dry_run: request.dry_run,
        input: request.bam.to_string_lossy().into_owned(),
        filters: FilterPolicy {
            mapped_only: request.mapped_only,
            unmapped_only: request.unmapped_only,
            primary_only: request.primary_only,
            length: LengthFilterPolicy {
                min: request.min_length,
                max: request.max_length,
            },
            mean_quality: MeanQualityFilterPolicy {
                min: request.min_mean_quality,
                max: request.max_mean_quality,
                missing_quality_policy: "drop_when_quality_filter_requested",
            },
            complexity: ComplexityFilterPolicy {
                min: request.min_complexity,
                max: request.max_complexity,
                k_min: complexity_requested.then_some(request.complexity_k_min),
                k_max: complexity_requested.then_some(request.complexity_k_max),
                method: "canonical_linguistic_complexity",
                noncanonical_policy: request.complexity_noncanonical,
            },
        },
        execution: FilterExecution {
            records_examined: counters.records_examined,
            records_retained: counters.records_retained,
            records_removed,
            order_preserved: true,
            quality_records_evaluated: counters.quality_records_evaluated,
            records_removed_missing_quality: counters.records_removed_missing_quality,
            complexity_records_evaluated: counters.complexity_records_evaluated,
            records_removed_noncanonical_complexity: counters
                .records_removed_noncanonical_complexity,
        },
        output: FilterOutput {
            path: request.out.to_string_lossy().into_owned(),
            written,
            overwritten,
            records_written: if written {
                counters.records_retained
            } else {
                0
            },
        },
        header: FilterHeaderPolicy {
            reference_dictionary_preserved: true,
            references_retained: header.header.references.len(),
            raw_header_preserved: true,
        },
        index,
        notes,
    }
}

fn output_index_candidate_paths(output: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    push_unique_path(
        &mut paths,
        PathBuf::from(format!("{}.bai", output.display())),
    );
    push_unique_path(
        &mut paths,
        PathBuf::from(format!("{}.csi", output.display())),
    );
    if let Some(file_name) = output.file_name() {
        push_unique_path(
            &mut paths,
            output.with_file_name(format!("{}.bai", file_name.to_string_lossy())),
        );
        push_unique_path(
            &mut paths,
            output.with_file_name(format!("{}.csi", file_name.to_string_lossy())),
        );
    }
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

fn temporary_output_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("filter-output.bam");
    output.with_file_name(format!(".{file_name}.tmp"))
}

fn existing_paths_are_same_file(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{
        bam::{
            records::{BAM_FUNMAP, RecordLayout, encode_bam_qualities, encode_bam_sequence},
            scan::BamScanner,
            write::{BgzfWriter, serialize_record_layout},
        },
        cli::FilterComplexityNonCanonicalPolicy,
        commands::filter::{FilterRequest, linguistic_complexity_bam, run},
        formats::bgzf::test_support::write_temp_file,
    };

    use crate::bam::header::{
        ReferenceHeaderFields, ReferenceRecord, serialize_bam_header_payload,
    };

    fn request(input: &PathBuf, output: &PathBuf) -> FilterRequest {
        FilterRequest {
            bam: input.clone(),
            out: output.clone(),
            min_length: None,
            max_length: None,
            min_mean_quality: None,
            max_mean_quality: None,
            min_complexity: None,
            max_complexity: None,
            complexity_k_min: 1,
            complexity_k_max: 4,
            complexity_noncanonical: FilterComplexityNonCanonicalPolicy::Drop,
            mapped_only: false,
            unmapped_only: false,
            primary_only: false,
            dry_run: false,
            force: true,
        }
    }

    fn record(
        read_name: &str,
        sequence: &str,
        qualities: &str,
        flags: u16,
        ref_id: i32,
    ) -> RecordLayout {
        RecordLayout {
            block_size: 0,
            ref_id,
            pos: if ref_id >= 0 { 0 } else { -1 },
            bin: 0,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags,
            mapping_quality: 60,
            n_cigar_op: 0,
            l_seq: sequence.len(),
            read_name: read_name.to_string(),
            cigar_bytes: Vec::new(),
            sequence_bytes: encode_bam_sequence(sequence).expect("sequence should encode"),
            quality_bytes: encode_bam_qualities(qualities).expect("qualities should encode"),
            aux_bytes: Vec::new(),
        }
    }

    fn write_filter_fixture(name: &str) -> PathBuf {
        let path = write_temp_file(name, "bam", b"placeholder");
        let header_payload = serialize_bam_header_payload(
            &path,
            "@SQ\tSN:chr1\tLN:100\n",
            &[ReferenceRecord {
                name: "chr1".to_string(),
                length: 100,
                index: 0,
                header_fields: ReferenceHeaderFields::default(),
                text_header_length: Some(100),
            }],
        )
        .expect("header should serialize");
        let records = [
            record("short_low", "ACGT", "!!!!", 0, 0),
            record("middle_high", "ACGTACGT", "IIIIIIII", 0, 0),
            record("long_low_complexity", "AAAAAAAAAAAA", "IIIIIIIIIIII", 0, 0),
            record("unmapped", "ACGTAC", "IIIIII", BAM_FUNMAP, -1),
        ];
        let mut writer = BgzfWriter::create(&path).expect("fixture should create");
        writer
            .write_all(&header_payload)
            .expect("header should write");
        for record in records {
            writer
                .write_all(&serialize_record_layout(&record))
                .expect("record should write");
        }
        writer.finish().expect("fixture should finish");
        path
    }

    fn output_names(path: &PathBuf) -> Vec<String> {
        let mut scanner = BamScanner::open(path).expect("output should scan");
        let mut names = Vec::new();
        while let Some(record) = scanner.next_record().expect("record should scan") {
            names.push(record.read_name().to_string());
        }
        names
    }

    #[test]
    fn filter_applies_length_quality_complexity_and_mapping_predicates() {
        let input = write_filter_fixture("filter-predicates");
        let output = input.with_extension("filtered.bam");
        let mut request = request(&input, &output);
        request.min_length = Some(6);
        request.max_length = Some(10);
        request.min_mean_quality = Some(30.0);
        request.min_complexity = Some(0.7);
        request.complexity_k_min = 1;
        request.complexity_k_max = 2;
        request.mapped_only = true;

        let payload = run(request).expect("filter should succeed");

        assert_eq!(payload.execution.records_examined, 4);
        assert_eq!(payload.execution.records_retained, 1);
        assert_eq!(payload.output.records_written, 1);
        assert_eq!(output_names(&output), vec!["middle_high"]);
        fs::remove_file(input).expect("input should remove");
        fs::remove_file(output).expect("output should remove");
    }

    #[test]
    fn dry_run_reports_counts_without_writing_output() {
        let input = write_filter_fixture("filter-dry-run");
        let output = input.with_extension("filtered.bam");
        let mut request = request(&input, &output);
        request.min_length = Some(5);
        request.dry_run = true;

        let payload = run(request).expect("dry run should succeed");

        assert_eq!(payload.execution.records_retained, 3);
        assert!(!payload.output.written);
        assert!(!output.exists());
        fs::remove_file(input).expect("input should remove");
    }

    #[test]
    fn linguistic_complexity_matches_emboss_rs_formula_for_canonical_bases() {
        let request = FilterRequest {
            bam: PathBuf::new(),
            out: PathBuf::new(),
            min_length: None,
            max_length: None,
            min_mean_quality: None,
            max_mean_quality: None,
            min_complexity: Some(0.0),
            max_complexity: None,
            complexity_k_min: 1,
            complexity_k_max: 2,
            complexity_noncanonical: FilterComplexityNonCanonicalPolicy::Drop,
            mapped_only: false,
            unmapped_only: false,
            primary_only: false,
            dry_run: true,
            force: false,
        };
        let encoded = encode_bam_sequence("ACGTACGT").expect("sequence should encode");
        let complexity =
            linguistic_complexity_bam(&encoded, 8, &request).expect("complexity should compute");

        assert!((complexity - (8.0 / 11.0)).abs() < 1e-12);
    }
}
