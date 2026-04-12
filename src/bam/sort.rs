use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    bam::{
        header::{
            parse_bam_header_from_reader, rewrite_header_for_sort, serialize_bam_header_payload,
        },
        reader::BamReader,
        records::{RecordLayout, read_next_record_layout},
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Coordinate,
    Queryname,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum QuerynameSubOrder {
    Natural,
    Lexicographical,
}

#[derive(Debug, Clone)]
pub struct SortExecutionOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub force: bool,
    pub order: SortOrder,
    pub queryname_suborder: Option<QuerynameSubOrder>,
    pub threads: usize,
    pub memory_limit: Option<u64>,
}

#[derive(Debug)]
pub struct SortExecution {
    pub overwritten: bool,
    pub records_read: u64,
    pub records_written: u64,
    pub produced_order: SortOrder,
    pub produced_sub_order: Option<QuerynameSubOrder>,
    pub notes: Vec<String>,
}

#[derive(Debug)]
struct SortableRecord {
    layout: RecordLayout,
    ordinal: u64,
}

pub fn sort_bam(options: &SortExecutionOptions) -> Result<SortExecution, AppError> {
    if output_matches_input(&options.input_path, &options.output_path) {
        return Err(AppError::WriteError {
            path: options.output_path.clone(),
            message: "Output path must differ from the input BAM path.".to_string(),
        });
    }

    let preexisting_output = options.output_path.exists();
    if preexisting_output && !options.force {
        return Err(AppError::OutputExists {
            path: options.output_path.clone(),
        });
    }

    let queryname_suborder = match (options.order, options.queryname_suborder) {
        (SortOrder::Coordinate, _) => None,
        (SortOrder::Queryname, Some(QuerynameSubOrder::Natural)) => {
            return Err(AppError::Unimplemented {
                path: options.input_path.clone(),
                detail:
                    "Queryname natural sorting is not implemented in this slice; use lexicographical ordering."
                        .to_string(),
            });
        }
        (SortOrder::Queryname, Some(QuerynameSubOrder::Lexicographical)) => {
            Some(QuerynameSubOrder::Lexicographical)
        }
        (SortOrder::Queryname, None) => Some(QuerynameSubOrder::Lexicographical),
    };

    let mut reader = BamReader::open(&options.input_path)?;
    let parsed_header = parse_bam_header_from_reader(&mut reader)?;
    let rewritten_header_text = rewrite_header_for_sort(
        &parsed_header.header.raw_header_text,
        sort_order_name(options.order),
        queryname_suborder_name(queryname_suborder),
    );
    let header_payload =
        serialize_bam_header_payload(&rewritten_header_text, &parsed_header.header.references);

    let mut records = Vec::new();
    let mut ordinal = 0_u64;
    loop {
        let Some(layout) = read_next_record_layout(&mut reader)? else {
            break;
        };
        records.push(SortableRecord { layout, ordinal });
        ordinal += 1;
    }

    sort_records(&mut records, options.order, queryname_suborder);

    let temp_path = temporary_output_path(&options.output_path);
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    let write_result = (|| -> Result<u64, AppError> {
        let mut writer = BgzfWriter::create(&temp_path)?;
        writer.write_all(&header_payload)?;
        let mut written = 0_u64;
        for record in &records {
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

    let mut notes = vec!["Initial implementation uses an in-memory sort strategy.".to_string()];
    if options.threads > 1 {
        notes.push(format!(
            "Thread count was set to {}, but this slice does not yet parallelize sorting or BGZF writing.",
            options.threads
        ));
    }
    if let Some(memory_limit) = options.memory_limit {
        notes.push(format!(
            "Memory limit {} bytes was accepted for future external-sort support and is not yet enforced by the current in-memory engine.",
            memory_limit
        ));
    }

    Ok(SortExecution {
        overwritten: preexisting_output && options.force,
        records_read: ordinal,
        records_written,
        produced_order: options.order,
        produced_sub_order: queryname_suborder,
        notes,
    })
}

fn sort_records(
    records: &mut [SortableRecord],
    order: SortOrder,
    queryname_suborder: Option<QuerynameSubOrder>,
) {
    match order {
        SortOrder::Coordinate => records.sort_by(compare_coordinate_records),
        SortOrder::Queryname => match queryname_suborder {
            Some(QuerynameSubOrder::Lexicographical) | None => {
                records.sort_by(compare_queryname_records)
            }
            Some(QuerynameSubOrder::Natural) => {}
        },
    }
}

fn compare_coordinate_records(left: &SortableRecord, right: &SortableRecord) -> Ordering {
    let left_unmapped = is_unmapped(&left.layout);
    let right_unmapped = is_unmapped(&right.layout);

    left_unmapped
        .cmp(&right_unmapped)
        .then_with(|| left.layout.ref_id.cmp(&right.layout.ref_id))
        .then_with(|| left.layout.pos.cmp(&right.layout.pos))
        .then_with(|| is_reverse(&left.layout).cmp(&is_reverse(&right.layout)))
        .then_with(|| left.layout.read_name.cmp(&right.layout.read_name))
        .then_with(|| left.layout.flags.cmp(&right.layout.flags))
        .then_with(|| left.layout.next_ref_id.cmp(&right.layout.next_ref_id))
        .then_with(|| left.layout.next_pos.cmp(&right.layout.next_pos))
        .then_with(|| left.layout.tlen.cmp(&right.layout.tlen))
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn compare_queryname_records(left: &SortableRecord, right: &SortableRecord) -> Ordering {
    left.layout
        .read_name
        .cmp(&right.layout.read_name)
        .then_with(|| left.layout.ref_id.cmp(&right.layout.ref_id))
        .then_with(|| left.layout.pos.cmp(&right.layout.pos))
        .then_with(|| left.layout.flags.cmp(&right.layout.flags))
        .then_with(|| left.layout.next_ref_id.cmp(&right.layout.next_ref_id))
        .then_with(|| left.layout.next_pos.cmp(&right.layout.next_pos))
        .then_with(|| left.layout.tlen.cmp(&right.layout.tlen))
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn output_matches_input(input: &Path, output: &Path) -> bool {
    if input == output {
        return true;
    }

    let input_canonical = fs::canonicalize(input).ok();
    let output_canonical = fs::canonicalize(output).ok();
    input_canonical.is_some() && input_canonical == output_canonical
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let stem = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-sort-output");
    output.with_file_name(format!(".{stem}.bamana-sort-{}.tmp", std::process::id()))
}

fn is_unmapped(record: &RecordLayout) -> bool {
    record.flags & 0x4 != 0 || record.ref_id < 0
}

fn is_reverse(record: &RecordLayout) -> u8 {
    u8::from(record.flags & 0x10 != 0)
}

fn sort_order_name(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Coordinate => "coordinate",
        SortOrder::Queryname => "queryname",
    }
}

fn queryname_suborder_name(suborder: Option<QuerynameSubOrder>) -> Option<&'static str> {
    match suborder {
        Some(QuerynameSubOrder::Natural) => Some("queryname:natural"),
        Some(QuerynameSubOrder::Lexicographical) => Some("queryname:lexicographical"),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bam::{
            header::parse_bam_header,
            sort::{QuerynameSubOrder, SortExecutionOptions, SortOrder, sort_bam},
        },
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    #[test]
    fn coordinate_sort_reorders_by_reference_then_position() {
        let input = write_temp_file(
            "sort-coordinate-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:unsorted\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(0, 9, "zread", 0),
                    build_light_record(0, 1, "aread", 0),
                ],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-sort-coordinate-output-{}.bam",
            std::process::id()
        ));

        let result = sort_bam(&SortExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            force: true,
            order: SortOrder::Coordinate,
            queryname_suborder: None,
            threads: 1,
            memory_limit: None,
        })
        .expect("sort should succeed");

        let header = parse_bam_header(&output).expect("output header should parse");
        assert_eq!(header.header.hd.sort_order.as_deref(), Some("coordinate"));
        assert_eq!(result.records_written, 2);

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn natural_queryname_sort_is_honestly_deferred() {
        let input = write_temp_file(
            "sort-natural-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "read10", 0)],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-sort-natural-output-{}.bam",
            std::process::id()
        ));

        let error = sort_bam(&SortExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            force: true,
            order: SortOrder::Queryname,
            queryname_suborder: Some(QuerynameSubOrder::Natural),
            threads: 1,
            memory_limit: None,
        })
        .expect_err("natural queryname sort should be deferred");

        assert_eq!(error.to_json_error().code, "unimplemented");

        fs::remove_file(input).expect("fixture should be removable");
    }
}
