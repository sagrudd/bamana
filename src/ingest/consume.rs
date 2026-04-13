use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    bam::{
        header::{ReferenceRecord, rewrite_header_for_sort, serialize_bam_header_payload},
        reader::BamReader,
        records::{RecordLayout, read_next_record_layout},
        sort::{compare_coordinate_layouts, compare_queryname_layouts},
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
    fastq::read_fastq_as_unmapped_records,
    formats::probe::DetectedFormat,
    ingest::{
        cram::{
            ConsumeReferenceContext, ConsumeReferencePolicy, ConsumeReferenceSourceUsed,
            normalize_cram_to_record_layouts, prepare_reference_context,
        },
        discovery::DiscoveredFile,
        sam::read_sam_file,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ConsumeMode {
    Alignment,
    Unmapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ConsumeSortOrder {
    None,
    Coordinate,
    Queryname,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConsumePlatform {
    Ont,
    Illumina,
    Pacbio,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSemanticClass {
    Alignment,
    RawRead,
    Unsupported,
}

#[derive(Debug)]
pub struct ConsumeExecutionOptions {
    pub mode: ConsumeMode,
    pub files: Vec<DiscoveredFile>,
    pub output_path: PathBuf,
    pub force: bool,
    pub sort: ConsumeSortOrder,
    pub reference: Option<PathBuf>,
    pub reference_cache: Option<PathBuf>,
    pub reference_policy: ConsumeReferencePolicy,
    pub sample: Option<String>,
    pub read_group: Option<String>,
    pub platform: Option<ConsumePlatform>,
}

#[derive(Debug)]
pub struct ConsumeExecution {
    pub records_written: u64,
    pub overwritten: bool,
    pub header_strategy: String,
    pub reference_compatibility: Option<String>,
    pub reference_source_used: Option<ConsumeReferenceSourceUsed>,
    pub decode_without_external_reference: Option<bool>,
    pub notes: Vec<String>,
}

pub fn classify_input_format(format: DetectedFormat) -> InputSemanticClass {
    match format {
        DetectedFormat::Bam | DetectedFormat::Sam | DetectedFormat::Cram => {
            InputSemanticClass::Alignment
        }
        DetectedFormat::Fastq | DetectedFormat::FastqGz => InputSemanticClass::RawRead,
        _ => InputSemanticClass::Unsupported,
    }
}

pub fn mapped_state_for_mode(mode: ConsumeMode) -> &'static str {
    match mode {
        ConsumeMode::Alignment => "mapped_or_mixed",
        ConsumeMode::Unmapped => "unmapped",
    }
}

pub fn header_strategy_for_mode(mode: ConsumeMode) -> &'static str {
    match mode {
        ConsumeMode::Alignment => "first_compatible_alignment_header",
        ConsumeMode::Unmapped => "synthetic_unmapped_header",
    }
}

pub fn prepare_cram_context_for_consume(
    output_path: &Path,
    policy: ConsumeReferencePolicy,
    reference: Option<&Path>,
    reference_cache: Option<&Path>,
    dry_run: bool,
) -> Result<ConsumeReferenceContext, AppError> {
    prepare_reference_context(output_path, policy, reference, reference_cache, dry_run)
}

pub fn execute_consume(options: &ConsumeExecutionOptions) -> Result<ConsumeExecution, AppError> {
    if options.output_path.exists() && !options.force {
        return Err(AppError::OutputExists {
            path: options.output_path.clone(),
        });
    }

    if output_matches_any_input(
        &options
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>(),
        &options.output_path,
    ) {
        return Err(AppError::InvalidConsumeRequest {
            path: options.output_path.clone(),
            detail: "Output path collides with one of the discovered input files.".to_string(),
        });
    }

    let cram_context = if matches!(options.mode, ConsumeMode::Alignment)
        && options
            .files
            .iter()
            .any(|file| matches!(file.detected_format, DetectedFormat::Cram))
    {
        Some(prepare_reference_context(
            &options.output_path,
            options.reference_policy,
            options.reference.as_deref(),
            options.reference_cache.as_deref(),
            false,
        )?)
    } else {
        None
    };

    match options.mode {
        ConsumeMode::Alignment => execute_alignment_consume(options, cram_context.as_ref()),
        ConsumeMode::Unmapped => execute_unmapped_consume(options),
    }
}

pub fn synthetic_unmapped_header(
    sample: Option<&str>,
    read_group: Option<&str>,
    platform: Option<ConsumePlatform>,
) -> String {
    let mut lines = vec!["@HD\tVN:1.6\tSO:unsorted".to_string()];
    if sample.is_some() || read_group.is_some() || platform.is_some() {
        let mut rg = String::from("@RG");
        let id = read_group.unwrap_or("rg1");
        rg.push_str("\tID:");
        rg.push_str(id);
        if let Some(sample) = sample {
            rg.push_str("\tSM:");
            rg.push_str(sample);
        }
        if let Some(platform) = platform {
            rg.push_str("\tPL:");
            rg.push_str(platform_name(platform));
        }
        lines.push(rg);
    }
    format!("{}\n", lines.join("\n"))
}

fn execute_alignment_consume(
    options: &ConsumeExecutionOptions,
    cram_context: Option<&ConsumeReferenceContext>,
) -> Result<ConsumeExecution, AppError> {
    let preexisting_output = options.output_path.exists();
    let mut base_header_text = None;
    let mut base_references: Option<Vec<ReferenceRecord>> = None;
    let mut header_strategy = header_strategy_for_mode(ConsumeMode::Alignment).to_string();
    let mut records = Vec::new();
    let mut notes = vec![
        "Alignment-bearing inputs were normalized into BAM.".to_string(),
        "Alignment compatibility currently requires identical reference dictionaries across all alignment inputs."
            .to_string(),
    ];
    let mut reference_source_used = None;
    let mut decode_without_external_reference = None;

    for file in &options.files {
        match file.detected_format {
            DetectedFormat::Bam => {
                let mut reader = BamReader::open(&file.path)?;
                let header = crate::bam::header::parse_bam_header_from_reader(&mut reader)?;

                if let Some(expected) = base_references.as_ref() {
                    ensure_compatible_reference_dictionary(
                        expected,
                        &header.header.references,
                        &file.path,
                    )?;
                } else {
                    base_header_text = Some(header.header.raw_header_text.clone());
                    base_references = Some(header.header.references.clone());
                }

                while let Some(layout) = read_next_record_layout(&mut reader)? {
                    records.push(layout);
                }
            }
            DetectedFormat::Cram => {
                let context = cram_context.ok_or_else(|| AppError::Internal {
                    message: "CRAM alignment input reached consume execution without a resolved reference context."
                        .to_string(),
                })?;
                let normalized = normalize_cram_to_record_layouts(&file.path, context)?;

                if let Some(expected) = base_references.as_ref() {
                    ensure_compatible_reference_dictionary(
                        expected,
                        &normalized.references,
                        &file.path,
                    )?;
                } else {
                    header_strategy = "decoded_cram_header".to_string();
                    base_header_text = Some(normalized.raw_header_text.clone());
                    base_references = Some(normalized.references.clone());
                }

                reference_source_used = Some(normalized.source_used);
                decode_without_external_reference =
                    Some(normalized.decode_without_external_reference);
                records.extend(normalized.records);
            }
            DetectedFormat::Sam => {
                let parsed = read_sam_file(&file.path)?;
                if let Some(expected) = base_references.as_ref() {
                    ensure_compatible_reference_dictionary(
                        expected,
                        &parsed.references,
                        &file.path,
                    )?;
                } else {
                    base_header_text = Some(parsed.raw_header_text.clone());
                    base_references = Some(parsed.references.clone());
                }
                records.extend(parsed.records);
            }
            other => {
                return Err(AppError::UnsupportedInputFormat {
                    path: file.path.clone(),
                    format: format!("Detected unsupported alignment input format {other}."),
                });
            }
        }
    }

    if let Some(context) = cram_context {
        notes.extend(context.notes.iter().cloned());

        match reference_source_used {
            Some(ConsumeReferenceSourceUsed::ExplicitFasta) => notes.push(
                "CRAM input was decoded under an explicit indexed FASTA reference policy."
                    .to_string(),
            ),
            Some(ConsumeReferenceSourceUsed::EmbeddedOrNotRequired) => notes.push(
                "CRAM decoding completed without an explicit external FASTA under the selected conservative policy."
                    .to_string(),
            ),
            None => {}
        }
    }

    let references = base_references.unwrap_or_default();
    let header_text = match options.sort {
        ConsumeSortOrder::None => {
            if options.files.len() > 1 {
                notes.push(
                    "Multiple alignment inputs were concatenated in deterministic discovery order, so @HD sort metadata was rewritten to unsorted."
                        .to_string(),
                );
                rewrite_header_for_sort(base_header_text.as_deref().unwrap_or(""), "unsorted", None)
            } else {
                base_header_text.unwrap_or_default()
            }
        }
        ConsumeSortOrder::Coordinate => {
            sort_records(&mut records, ConsumeSortOrder::Coordinate);
            notes.push(
                "Post-ingest coordinate sorting reused Bamana's in-memory comparator family in this slice."
                    .to_string(),
            );
            rewrite_header_for_sort(
                base_header_text.as_deref().unwrap_or(""),
                "coordinate",
                None,
            )
        }
        ConsumeSortOrder::Queryname => {
            sort_records(&mut records, ConsumeSortOrder::Queryname);
            notes.push(
                "Post-ingest queryname sorting reused Bamana's lexicographical in-memory comparator family in this slice."
                    .to_string(),
            );
            rewrite_header_for_sort(
                base_header_text.as_deref().unwrap_or(""),
                "queryname",
                Some("queryname:lexicographical"),
            )
        }
    };

    let records_written = write_output_bam(
        &options.output_path,
        options.force,
        &header_text,
        &references,
        &records,
    )?;

    Ok(ConsumeExecution {
        records_written,
        overwritten: preexisting_output && options.force,
        header_strategy,
        reference_compatibility: Some("compatible".to_string()),
        reference_source_used,
        decode_without_external_reference,
        notes,
    })
}

fn execute_unmapped_consume(
    options: &ConsumeExecutionOptions,
) -> Result<ConsumeExecution, AppError> {
    let preexisting_output = options.output_path.exists();
    if matches!(options.sort, ConsumeSortOrder::Coordinate) {
        return Err(AppError::InvalidConsumeRequest {
            path: options.output_path.clone(),
            detail:
                "Coordinate sort is not semantically valid for Stage 1 unmapped consume output."
                    .to_string(),
        });
    }

    let mut records = Vec::new();
    for file in &options.files {
        match file.detected_format {
            DetectedFormat::Fastq | DetectedFormat::FastqGz => {
                let mut file_records =
                    read_fastq_as_unmapped_records(&file.path, options.read_group.as_deref())?;
                records.append(&mut file_records);
            }
            other => {
                return Err(AppError::UnsupportedInputFormat {
                    path: file.path.clone(),
                    format: format!("Detected unsupported raw-read input format {other}."),
                });
            }
        }
    }

    let mut header_text = synthetic_unmapped_header(
        options.sample.as_deref(),
        options.read_group.as_deref(),
        options.platform,
    );
    let mut notes = vec![
        "FASTQ inputs were converted to unmapped BAM records.".to_string(),
        "No alignments were inferred or fabricated during unmapped ingestion.".to_string(),
    ];

    if matches!(options.sort, ConsumeSortOrder::Queryname) {
        sort_records(&mut records, ConsumeSortOrder::Queryname);
        header_text =
            rewrite_header_for_sort(&header_text, "queryname", Some("queryname:lexicographical"));
        notes.push(
            "Unmapped records were queryname-sorted using Bamana's lexicographical comparator family in this slice."
                .to_string(),
        );
    }

    let records_written = write_output_bam(
        &options.output_path,
        options.force,
        &header_text,
        &[],
        &records,
    )?;

    Ok(ConsumeExecution {
        records_written,
        overwritten: preexisting_output && options.force,
        header_strategy: header_strategy_for_mode(ConsumeMode::Unmapped).to_string(),
        reference_compatibility: None,
        reference_source_used: None,
        decode_without_external_reference: None,
        notes,
    })
}

fn sort_records(records: &mut [RecordLayout], sort: ConsumeSortOrder) {
    match sort {
        ConsumeSortOrder::None => {}
        ConsumeSortOrder::Coordinate => {
            records.sort_by(|left, right| compare_coordinate_layouts(left, 0, right, 0))
        }
        ConsumeSortOrder::Queryname => {
            records.sort_by(|left, right| compare_queryname_layouts(left, 0, right, 0))
        }
    }
}

fn ensure_compatible_reference_dictionary(
    expected: &[ReferenceRecord],
    observed: &[ReferenceRecord],
    input_path: &Path,
) -> Result<(), AppError> {
    if expected.len() != observed.len() {
        return Err(AppError::IncompatibleHeaders {
            path: input_path.to_path_buf(),
            detail: format!(
                "Reference dictionary count differs between inputs: expected {}, observed {}.",
                expected.len(),
                observed.len()
            ),
        });
    }

    for (index, (left, right)) in expected.iter().zip(observed.iter()).enumerate() {
        if left.name != right.name {
            return Err(AppError::IncompatibleHeaders {
                path: input_path.to_path_buf(),
                detail: format!(
                    "Reference dictionary mismatch at index {index}: expected {}, observed {}.",
                    left.name, right.name
                ),
            });
        }
        if left.length != right.length {
            return Err(AppError::IncompatibleHeaders {
                path: input_path.to_path_buf(),
                detail: format!(
                    "Reference dictionary mismatch at index {index}: {} length differs between inputs ({} vs {}).",
                    left.name, left.length, right.length
                ),
            });
        }
    }

    Ok(())
}

fn write_output_bam(
    output_path: &Path,
    force: bool,
    header_text: &str,
    references: &[ReferenceRecord],
    records: &[RecordLayout],
) -> Result<u64, AppError> {
    let preexisting_output = output_path.exists();
    let header_payload = serialize_bam_header_payload(header_text, references);
    let temp_path = temporary_output_path(output_path);
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    let write_result = (|| -> Result<u64, AppError> {
        let mut writer = BgzfWriter::create(&temp_path)?;
        writer.write_all(&header_payload)?;
        let mut written = 0_u64;
        for record in records {
            writer.write_all(&serialize_record_layout(record))?;
            written += 1;
        }
        writer.finish()?;
        Ok(written)
    })();

    let records_written = match write_result {
        Ok(records_written) => records_written,
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
    };

    if preexisting_output && force {
        fs::remove_file(output_path).map_err(|error| AppError::WriteError {
            path: output_path.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::rename(&temp_path, output_path).map_err(|error| AppError::WriteError {
        path: output_path.to_path_buf(),
        message: error.to_string(),
    })?;

    Ok(records_written)
}

fn output_matches_any_input(inputs: &[PathBuf], output: &Path) -> bool {
    inputs.iter().any(|input| same_path(input, output))
}

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    let left_canonical = fs::canonicalize(left).ok();
    let right_canonical = fs::canonicalize(right).ok();
    left_canonical.is_some() && left_canonical == right_canonical
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let stem = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-consume-output");
    output.with_file_name(format!(".{stem}.bamana-consume-{}.tmp", std::process::id()))
}

fn platform_name(platform: ConsumePlatform) -> &'static str {
    match platform {
        ConsumePlatform::Ont => "ONT",
        ConsumePlatform::Illumina => "ILLUMINA",
        ConsumePlatform::Pacbio => "PACBIO",
        ConsumePlatform::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bam::{header::parse_bam_header, reader::BamReader, records::read_next_record_layout},
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
        ingest::{
            consume::{
                ConsumeExecutionOptions, ConsumeMode, ConsumePlatform, ConsumeSortOrder,
                execute_consume, synthetic_unmapped_header,
            },
            discovery::DiscoveredFile,
        },
    };

    #[test]
    fn synthetic_header_includes_requested_rg_metadata() {
        let header =
            synthetic_unmapped_header(Some("sample1"), Some("rg1"), Some(ConsumePlatform::Ont));
        assert!(header.contains("@HD"));
        assert!(header.contains("@RG\tID:rg1\tSM:sample1\tPL:ONT"));
    }

    #[test]
    fn alignment_consume_normalizes_bam_input_to_bam_output() {
        let input = write_temp_file(
            "consume-alignment-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "read1", 0)],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-consume-alignment-output-{}.bam",
            std::process::id()
        ));

        let execution = execute_consume(&ConsumeExecutionOptions {
            mode: ConsumeMode::Alignment,
            files: vec![DiscoveredFile {
                path: input.clone(),
                detected_format: crate::formats::probe::DetectedFormat::Bam,
            }],
            output_path: output.clone(),
            force: true,
            sort: ConsumeSortOrder::None,
            reference: None,
            reference_cache: None,
            reference_policy: crate::ingest::cram::ConsumeReferencePolicy::Strict,
            sample: None,
            read_group: None,
            platform: None,
        })
        .expect("alignment consume should succeed");

        assert_eq!(execution.records_written, 1);
        let reparsed = crate::bam::header::parse_bam_header(&output).expect("output should parse");
        assert_eq!(reparsed.header.references.len(), 1);

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn unmapped_consume_normalizes_fastq_input_to_bam_output() {
        let input = std::env::temp_dir().join(format!(
            "bamana-consume-fastq-input-{}.fastq",
            std::process::id()
        ));
        fs::write(&input, "@zread\nAC\n+\n!!\n@aread\nGT\n+\n##\n")
            .expect("fastq fixture should write");
        let output = std::env::temp_dir().join(format!(
            "bamana-consume-unmapped-output-{}.bam",
            std::process::id()
        ));

        let execution = execute_consume(&ConsumeExecutionOptions {
            mode: ConsumeMode::Unmapped,
            files: vec![DiscoveredFile {
                path: input.clone(),
                detected_format: crate::formats::probe::DetectedFormat::Fastq,
            }],
            output_path: output.clone(),
            force: true,
            sort: ConsumeSortOrder::Queryname,
            reference: None,
            reference_cache: None,
            reference_policy: crate::ingest::cram::ConsumeReferencePolicy::Strict,
            sample: Some("sample1".to_string()),
            read_group: Some("rg1".to_string()),
            platform: Some(ConsumePlatform::Illumina),
        })
        .expect("unmapped consume should succeed");

        assert_eq!(execution.records_written, 2);
        let header = parse_bam_header(&output).expect("output header should parse");
        assert_eq!(header.header.hd.sort_order.as_deref(), Some("queryname"));
        assert_eq!(header.header.read_groups.len(), 1);

        let mut reader = BamReader::open(&output).expect("output BAM should reopen");
        let _ = crate::bam::header::parse_bam_header_from_reader(&mut reader)
            .expect("output header should parse from reader");
        let first = read_next_record_layout(&mut reader)
            .expect("record read should succeed")
            .expect("first record should exist");
        let second = read_next_record_layout(&mut reader)
            .expect("record read should succeed")
            .expect("second record should exist");
        assert_eq!(first.read_name, "aread");
        assert_eq!(second.read_name, "zread");

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }
}
