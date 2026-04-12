use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    bam::{
        header::{
            ReferenceRecord, parse_bam_header_from_reader, rewrite_header_for_sort,
            serialize_bam_header_payload,
        },
        reader::BamReader,
        records::{RecordLayout, read_next_record_layout},
        sort::{
            QuerynameSubOrder, compare_coordinate_layouts, compare_queryname_layouts,
            queryname_suborder_name,
        },
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum MergeMode {
    Input,
    Coordinate,
    Queryname,
}

#[derive(Debug, Clone)]
pub struct MergeExecutionOptions {
    pub input_paths: Vec<PathBuf>,
    pub output_path: PathBuf,
    pub force: bool,
    pub mode: MergeMode,
    pub queryname_suborder: Option<QuerynameSubOrder>,
    pub threads: usize,
}

#[derive(Debug)]
pub struct MergeExecution {
    pub per_input_records_read: Vec<u64>,
    pub records_written: u64,
    pub produced_mode: MergeMode,
    pub produced_sub_order: Option<QuerynameSubOrder>,
    pub overwritten: bool,
    pub notes: Vec<String>,
    pub records_for_checksum: Vec<RecordLayout>,
}

#[derive(Debug)]
struct MergeableRecord {
    layout: RecordLayout,
    ordinal: u64,
}

pub fn merge_bams(options: &MergeExecutionOptions) -> Result<MergeExecution, AppError> {
    if options.input_paths.is_empty() {
        return Err(AppError::InvalidMergeRequest {
            path: options.output_path.clone(),
            detail: "At least one input BAM is required for merge.".to_string(),
        });
    }

    let mode = resolve_mode(
        options.mode,
        options.queryname_suborder,
        &options.output_path,
    )?;
    let preexisting_output = options.output_path.exists();
    if preexisting_output && !options.force {
        return Err(AppError::OutputExists {
            path: options.output_path.clone(),
        });
    }

    if output_matches_any_input(&options.input_paths, &options.output_path) {
        return Err(AppError::WriteError {
            path: options.output_path.clone(),
            message: "Output path must differ from all input BAM paths.".to_string(),
        });
    }

    let mut base_header_text = None;
    let mut base_references: Option<Vec<ReferenceRecord>> = None;
    let mut merged_records = Vec::new();
    let mut per_input_records_read = Vec::with_capacity(options.input_paths.len());
    let mut global_ordinal = 0_u64;

    for (input_index, input_path) in options.input_paths.iter().enumerate() {
        let mut reader = BamReader::open(input_path)?;
        let header = parse_bam_header_from_reader(&mut reader)?;

        if let Some(expected) = base_references.as_ref() {
            ensure_compatible_reference_dictionary(
                expected,
                &header.header.references,
                input_path,
            )?;
        } else {
            base_header_text = Some(header.header.raw_header_text.clone());
            base_references = Some(header.header.references.clone());
        }

        let mut input_records = 0_u64;
        loop {
            let Some(layout) = read_next_record_layout(&mut reader)? else {
                break;
            };
            merged_records.push(MergeableRecord {
                layout,
                ordinal: global_ordinal,
            });
            global_ordinal += 1;
            input_records += 1;
        }

        let _ = input_index;
        per_input_records_read.push(input_records);
    }

    let produced_sub_order = match mode {
        MergeMode::Queryname => options
            .queryname_suborder
            .or(Some(QuerynameSubOrder::Lexicographical)),
        _ => None,
    };

    match mode {
        MergeMode::Input => {}
        MergeMode::Coordinate => {
            merged_records.sort_by(|left, right| {
                compare_coordinate_layouts(&left.layout, left.ordinal, &right.layout, right.ordinal)
            });
        }
        MergeMode::Queryname => {
            merged_records.sort_by(|left, right| {
                compare_queryname_layouts(&left.layout, left.ordinal, &right.layout, right.ordinal)
            });
        }
    }

    let rewritten_header = rewrite_header_for_merge(
        base_header_text
            .as_deref()
            .expect("first input header should have been captured"),
        mode,
        produced_sub_order,
    );
    let header_payload = serialize_bam_header_payload(
        &rewritten_header,
        base_references
            .as_ref()
            .expect("first input references should have been captured"),
    );

    let temp_path = temporary_output_path(&options.output_path, "merge");
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    let write_result = (|| -> Result<u64, AppError> {
        let mut writer = BgzfWriter::create(&temp_path)?;
        writer.write_all(&header_payload)?;
        let mut written = 0_u64;
        for record in &merged_records {
            writer.write_all(&serialize_record_layout(&record.layout))?;
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

    if preexisting_output && options.force {
        fs::remove_file(&options.output_path).map_err(|error| AppError::WriteError {
            path: options.output_path.clone(),
            message: error.to_string(),
        })?;
    }
    fs::rename(&temp_path, &options.output_path).map_err(|error| AppError::WriteError {
        path: options.output_path.clone(),
        message: error.to_string(),
    })?;

    let mut notes = vec![
        "Initial implementation uses an in-memory merge strategy.".to_string(),
        "Merge compatibility currently requires identical binary reference dictionaries across all inputs."
            .to_string(),
        "The first input header is used as the merge base and only @HD sort metadata is rewritten in this slice."
            .to_string(),
    ];
    if options.threads > 1 {
        notes.push(format!(
            "Thread count was set to {}, but this slice does not yet parallelize merge collection or BGZF writing.",
            options.threads
        ));
    }
    match mode {
        MergeMode::Input => {
            notes.push("Output preserves input-file concatenation order.".to_string())
        }
        MergeMode::Coordinate => notes
            .push("Coordinate merge used the same comparator family as bamana sort.".to_string()),
        MergeMode::Queryname => notes.push(
            "Queryname merge used the same lexicographical comparator family as bamana sort."
                .to_string(),
        ),
    }

    Ok(MergeExecution {
        per_input_records_read,
        records_written,
        produced_mode: mode,
        produced_sub_order,
        overwritten: preexisting_output && options.force,
        notes,
        records_for_checksum: merged_records
            .into_iter()
            .map(|record| record.layout)
            .collect(),
    })
}

fn resolve_mode(
    mode: MergeMode,
    queryname_suborder: Option<QuerynameSubOrder>,
    path: &Path,
) -> Result<MergeMode, AppError> {
    if matches!(mode, MergeMode::Input | MergeMode::Coordinate) && queryname_suborder.is_some() {
        return Err(AppError::InvalidMergeRequest {
            path: path.to_path_buf(),
            detail:
                "A queryname sub-order can only be requested when merge output order is queryname."
                    .to_string(),
        });
    }
    if matches!(queryname_suborder, Some(QuerynameSubOrder::Natural)) {
        return Err(AppError::Unimplemented {
            path: path.to_path_buf(),
            detail:
                "Queryname natural ordering is not implemented in this slice; use lexicographical ordering."
                    .to_string(),
        });
    }
    Ok(mode)
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

fn rewrite_header_for_merge(
    raw_header_text: &str,
    mode: MergeMode,
    queryname_suborder: Option<QuerynameSubOrder>,
) -> String {
    match mode {
        MergeMode::Input => rewrite_header_for_sort(raw_header_text, "unsorted", None),
        MergeMode::Coordinate => rewrite_header_for_sort(raw_header_text, "coordinate", None),
        MergeMode::Queryname => rewrite_header_for_sort(
            raw_header_text,
            "queryname",
            queryname_suborder_name(queryname_suborder),
        ),
    }
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

fn temporary_output_path(output: &Path, operation: &str) -> PathBuf {
    let stem = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-output");
    output.with_file_name(format!(
        ".{stem}.bamana-{operation}-{}.tmp",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bam::{
            checksum::{ChecksumFilters, compute_canonical_digest_for_records},
            merge::{MergeExecutionOptions, MergeMode, merge_bams},
        },
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    #[test]
    fn input_merge_preserves_concatenation_counts() {
        let input_a = write_temp_file(
            "merge-input-a",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "a", 0)],
            ),
        );
        let input_b = write_temp_file(
            "merge-input-b",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 2, "b", 0)],
            ),
        );
        let output =
            std::env::temp_dir().join(format!("bamana-merge-output-{}.bam", std::process::id()));

        let result = merge_bams(&MergeExecutionOptions {
            input_paths: vec![input_a.clone(), input_b.clone()],
            output_path: output.clone(),
            force: true,
            mode: MergeMode::Input,
            queryname_suborder: None,
            threads: 1,
        })
        .expect("merge should succeed");

        assert_eq!(result.per_input_records_read, vec![1, 1]);
        assert_eq!(result.records_written, 2);

        fs::remove_file(input_a).expect("fixture should be removable");
        fs::remove_file(input_b).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn incompatible_reference_dictionaries_fail_merge() {
        let input_a = write_temp_file(
            "merge-incompat-a",
            "bam",
            &build_bam_file_with_header_and_records("@SQ\tSN:chr1\tLN:10\n", &[("chr1", 10)], &[]),
        );
        let input_b = write_temp_file(
            "merge-incompat-b",
            "bam",
            &build_bam_file_with_header_and_records("@SQ\tSN:chr1\tLN:11\n", &[("chr1", 11)], &[]),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-merge-incompat-output-{}.bam",
            std::process::id()
        ));

        let error = merge_bams(&MergeExecutionOptions {
            input_paths: vec![input_a.clone(), input_b.clone()],
            output_path: output,
            force: true,
            mode: MergeMode::Input,
            queryname_suborder: None,
            threads: 1,
        })
        .expect_err("merge should fail");

        assert_eq!(error.to_json_error().code, "incompatible_headers");

        fs::remove_file(input_a).expect("fixture should be removable");
        fs::remove_file(input_b).expect("fixture should be removable");
    }

    #[test]
    fn canonical_digest_over_merged_records_is_order_insensitive() {
        let record_a = build_light_record(0, 2, "b", 0);
        let record_b = build_light_record(0, 1, "a", 0);
        let input_a = write_temp_file(
            "merge-digest-a",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[record_a],
            ),
        );
        let input_b = write_temp_file(
            "merge-digest-b",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[record_b],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-merge-digest-output-{}.bam",
            std::process::id()
        ));

        let result = merge_bams(&MergeExecutionOptions {
            input_paths: vec![input_a.clone(), input_b.clone()],
            output_path: output.clone(),
            force: true,
            mode: MergeMode::Coordinate,
            queryname_suborder: None,
            threads: 1,
        })
        .expect("merge should succeed");

        let digest = compute_canonical_digest_for_records(
            &result.records_for_checksum,
            ChecksumFilters {
                only_primary: false,
                mapped_only: false,
            },
            &std::collections::HashSet::new(),
        )
        .expect("canonical digest should succeed");
        assert!(!digest.is_empty());

        fs::remove_file(input_a).expect("fixture should be removable");
        fs::remove_file(input_b).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }
}
