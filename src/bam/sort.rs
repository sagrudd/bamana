use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use rayon::prelude::*;
use serde::Serialize;

use crate::{
    bam::{
        header::{rewrite_header_for_sort, serialize_bam_header_payload},
        records::RecordLayout,
        scan::BamScanner,
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
    output_safety::{finalize_completed_output, remove_stale_temp},
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

    let mut scanner = BamScanner::open(&options.input_path)?;
    let parsed_header = scanner.header();
    let rewritten_header_text = rewrite_header_for_sort(
        &parsed_header.header.raw_header_text,
        sort_order_name(options.order),
        queryname_suborder_name(queryname_suborder),
    );
    let header_payload = serialize_bam_header_payload(
        &options.output_path,
        &rewritten_header_text,
        &parsed_header.header.references,
    )?;

    if let Some(memory_limit) = options.memory_limit {
        return external_sort_bam(
            options,
            queryname_suborder,
            header_payload,
            scanner,
            memory_limit,
            preexisting_output,
        );
    }

    let mut records = Vec::new();
    let mut ordinal = 0_u64;
    while let Some(record) = scanner.next_record()? {
        records.push(SortableRecord {
            layout: record.to_record_layout(),
            ordinal,
        });
        ordinal += 1;
    }

    if ordinal != scanner.records_read() {
        return Err(AppError::InvalidRecord {
            path: options.input_path.clone(),
            detail: "Sort scanner record count diverged from records materialized for sorting."
                .to_string(),
        });
    }

    sort_records(
        &mut records,
        options.order,
        queryname_suborder,
        options.threads,
    )?;

    let temp_path = temporary_output_path(&options.output_path);
    remove_stale_temp(&temp_path);

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

    finalize_completed_output(&temp_path, &options.output_path, options.force)?;

    let notes = vec![
        "Sort used the in-memory strategy because no memory limit was supplied.".to_string(),
        format!(
            "Record ordering used {} worker thread(s); BGZF output compression remains ordered and single-stream.",
            options.threads.max(1)
        ),
    ];

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
    threads: usize,
) -> Result<(), AppError> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .build()
        .map_err(|error| AppError::InvalidSortRequest {
            path: PathBuf::from("<sort-worker-pool>"),
            detail: format!("Could not create the requested sort worker pool: {error}"),
        })?;
    pool.install(|| match order {
        SortOrder::Coordinate => records.par_sort_by(compare_coordinate_records),
        SortOrder::Queryname => match queryname_suborder {
            Some(QuerynameSubOrder::Lexicographical) | None => {
                records.par_sort_by(compare_queryname_records)
            }
            Some(QuerynameSubOrder::Natural) => {}
        },
    });
    Ok(())
}

fn external_sort_bam(
    options: &SortExecutionOptions,
    queryname_suborder: Option<QuerynameSubOrder>,
    header_payload: Vec<u8>,
    mut scanner: BamScanner,
    memory_limit: u64,
    preexisting_output: bool,
) -> Result<SortExecution, AppError> {
    if memory_limit == 0 {
        return Err(AppError::InvalidSortRequest {
            path: options.input_path.clone(),
            detail: "Sort memory limit must be greater than zero.".to_string(),
        });
    }

    let temp_path = temporary_output_path(&options.output_path);
    remove_stale_temp(&temp_path);
    let mut runs = TemporaryRuns::default();
    let mut records = Vec::new();
    let mut estimated_bytes = 0_u64;
    let mut ordinal = 0_u64;

    while let Some(record) = scanner.next_record()? {
        let layout = record.to_record_layout();
        estimated_bytes = estimated_bytes.saturating_add(estimated_record_bytes(&layout));
        records.push(SortableRecord { layout, ordinal });
        ordinal += 1;

        if estimated_bytes >= memory_limit {
            spill_run(
                options,
                queryname_suborder,
                &header_payload,
                &mut records,
                &mut runs,
            )?;
            estimated_bytes = 0;
        }
    }

    if ordinal != scanner.records_read() {
        return Err(AppError::InvalidRecord {
            path: options.input_path.clone(),
            detail:
                "External-sort scanner record count diverged from records materialized for sorting."
                    .to_string(),
        });
    }
    if !records.is_empty() {
        spill_run(
            options,
            queryname_suborder,
            &header_payload,
            &mut records,
            &mut runs,
        )?;
    }

    let records_written = merge_runs(
        &runs.paths,
        &temp_path,
        &header_payload,
        options.order,
        queryname_suborder,
    )
    .inspect_err(|_| {
        let _ = fs::remove_file(&temp_path);
    })?;
    if records_written != ordinal {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::InvalidRecord {
            path: options.input_path.clone(),
            detail: format!(
                "External sort read {ordinal} records but merged {records_written} records."
            ),
        });
    }

    finalize_completed_output(&temp_path, &options.output_path, options.force)?;
    let run_count = runs.paths.len();
    runs.remove_all();

    Ok(SortExecution {
        overwritten: preexisting_output && options.force,
        records_read: ordinal,
        records_written,
        produced_order: options.order,
        produced_sub_order: queryname_suborder,
        notes: vec![
            format!(
                "Sort used a bounded external merge with {run_count} temporary run(s) and a {memory_limit}-byte target memory budget."
            ),
            format!(
                "Run ordering used {} worker thread(s); the stable multiway merge and ordered BGZF output remained deterministic.",
                options.threads.max(1)
            ),
        ],
    })
}

fn estimated_record_bytes(layout: &RecordLayout) -> u64 {
    const RECORD_OVERHEAD: u64 = 128;
    (layout.block_size as u64)
        .saturating_add(4)
        .saturating_add(RECORD_OVERHEAD)
}

fn spill_run(
    options: &SortExecutionOptions,
    queryname_suborder: Option<QuerynameSubOrder>,
    header_payload: &[u8],
    records: &mut Vec<SortableRecord>,
    runs: &mut TemporaryRuns,
) -> Result<(), AppError> {
    sort_records(records, options.order, queryname_suborder, options.threads)?;
    let path = temporary_run_path(&options.output_path, runs.paths.len());
    remove_stale_temp(&path);
    let write_result = (|| -> Result<(), AppError> {
        let mut writer = BgzfWriter::create(&path)?;
        writer.write_all(header_payload)?;
        for record in records.iter() {
            writer.write_all(&serialize_record_layout(&record.layout))?;
        }
        writer.finish()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    runs.paths.push(path);
    records.clear();
    Ok(())
}

fn merge_runs(
    run_paths: &[PathBuf],
    output_path: &Path,
    header_payload: &[u8],
    order: SortOrder,
    queryname_suborder: Option<QuerynameSubOrder>,
) -> Result<u64, AppError> {
    let mut scanners = run_paths
        .iter()
        .map(|path| BamScanner::open(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut heap = BinaryHeap::new();
    for (run_index, scanner) in scanners.iter_mut().enumerate() {
        if let Some(record) = scanner.next_record()? {
            heap.push(MergeItem {
                layout: record.to_record_layout(),
                run_index,
                sequence: 0,
                order,
                queryname_suborder,
            });
        }
    }

    let write_result = (|| -> Result<u64, AppError> {
        let mut writer = BgzfWriter::create(output_path)?;
        writer.write_all(header_payload)?;
        let mut written = 0_u64;
        while let Some(item) = heap.pop() {
            writer.write_all(&serialize_record_layout(&item.layout))?;
            written += 1;
            let next_sequence = item.sequence + 1;
            if let Some(record) = scanners[item.run_index].next_record()? {
                heap.push(MergeItem {
                    layout: record.to_record_layout(),
                    run_index: item.run_index,
                    sequence: next_sequence,
                    order,
                    queryname_suborder,
                });
            }
        }
        writer.finish()?;
        Ok(written)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(output_path);
    }
    write_result
}

#[derive(Debug)]
struct MergeItem {
    layout: RecordLayout,
    run_index: usize,
    sequence: u64,
    order: SortOrder,
    queryname_suborder: Option<QuerynameSubOrder>,
}

impl PartialEq for MergeItem {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for MergeItem {}

impl PartialOrd for MergeItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeItem {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_layouts(
            &other.layout,
            &self.layout,
            self.order,
            self.queryname_suborder,
        )
        .then_with(|| other.run_index.cmp(&self.run_index))
        .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

fn compare_layouts(
    left: &RecordLayout,
    right: &RecordLayout,
    order: SortOrder,
    queryname_suborder: Option<QuerynameSubOrder>,
) -> Ordering {
    match order {
        SortOrder::Coordinate => compare_coordinate_layouts(left, 0, right, 0),
        SortOrder::Queryname => match queryname_suborder {
            Some(QuerynameSubOrder::Lexicographical) | None => {
                compare_queryname_layouts(left, 0, right, 0)
            }
            Some(QuerynameSubOrder::Natural) => Ordering::Equal,
        },
    }
}

fn temporary_run_path(output: &Path, run_index: usize) -> PathBuf {
    let stem = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-sort-output");
    output.with_file_name(format!(
        ".{stem}.bamana-sort-{}.run-{run_index:06}.bam",
        std::process::id()
    ))
}

#[derive(Default)]
struct TemporaryRuns {
    paths: Vec<PathBuf>,
}

impl TemporaryRuns {
    fn remove_all(&mut self) {
        for path in self.paths.drain(..) {
            let _ = fs::remove_file(path);
        }
    }
}

impl Drop for TemporaryRuns {
    fn drop(&mut self) {
        self.remove_all();
    }
}

fn compare_coordinate_records(left: &SortableRecord, right: &SortableRecord) -> Ordering {
    compare_coordinate_layouts(&left.layout, left.ordinal, &right.layout, right.ordinal)
}

fn compare_queryname_records(left: &SortableRecord, right: &SortableRecord) -> Ordering {
    compare_queryname_layouts(&left.layout, left.ordinal, &right.layout, right.ordinal)
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

pub(crate) fn compare_coordinate_layouts(
    left: &RecordLayout,
    left_ordinal: u64,
    right: &RecordLayout,
    right_ordinal: u64,
) -> Ordering {
    let left_unmapped = is_unmapped(left);
    let right_unmapped = is_unmapped(right);

    left_unmapped
        .cmp(&right_unmapped)
        .then_with(|| left.ref_id.cmp(&right.ref_id))
        .then_with(|| left.pos.cmp(&right.pos))
        .then_with(|| is_reverse(left).cmp(&is_reverse(right)))
        .then_with(|| left.read_name.cmp(&right.read_name))
        .then_with(|| left.flags.cmp(&right.flags))
        .then_with(|| left.next_ref_id.cmp(&right.next_ref_id))
        .then_with(|| left.next_pos.cmp(&right.next_pos))
        .then_with(|| left.tlen.cmp(&right.tlen))
        .then_with(|| left_ordinal.cmp(&right_ordinal))
}

pub(crate) fn compare_queryname_layouts(
    left: &RecordLayout,
    left_ordinal: u64,
    right: &RecordLayout,
    right_ordinal: u64,
) -> Ordering {
    left.read_name
        .cmp(&right.read_name)
        .then_with(|| left.ref_id.cmp(&right.ref_id))
        .then_with(|| left.pos.cmp(&right.pos))
        .then_with(|| left.flags.cmp(&right.flags))
        .then_with(|| left.next_ref_id.cmp(&right.next_ref_id))
        .then_with(|| left.next_pos.cmp(&right.next_pos))
        .then_with(|| left.tlen.cmp(&right.tlen))
        .then_with(|| left_ordinal.cmp(&right_ordinal))
}

fn is_unmapped(record: &RecordLayout) -> bool {
    record.flags & 0x4 != 0 || record.ref_id < 0
}

fn is_reverse(record: &RecordLayout) -> u8 {
    u8::from(record.flags & 0x10 != 0)
}

pub(crate) fn sort_order_name(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Coordinate => "coordinate",
        SortOrder::Queryname => "queryname",
    }
}

pub(crate) fn queryname_suborder_name(suborder: Option<QuerynameSubOrder>) -> Option<&'static str> {
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
            scan::BamScanner,
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
        let records = read_sorted_records(&output);
        assert_eq!(record_names(&records), vec!["aread", "zread"]);
        assert_eq!(
            records.iter().map(|record| record.pos).collect::<Vec<_>>(),
            vec![1, 9]
        );

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn coordinate_sort_places_mapped_before_unmapped_and_uses_stable_ties() {
        let input = write_temp_file(
            "sort-coordinate-unmapped-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:unsorted\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(-1, -1, "unmapped", 0x4),
                    build_light_record(0, 5, "reverse", 0x10),
                    build_light_record(0, 5, "forward", 0),
                    build_light_record(0, 5, "forward2", 0),
                ],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-sort-coordinate-unmapped-output-{}.bam",
            std::process::id()
        ));

        sort_bam(&SortExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            force: true,
            order: SortOrder::Coordinate,
            queryname_suborder: None,
            threads: 1,
            memory_limit: None,
        })
        .expect("sort should succeed");

        let records = read_sorted_records(&output);
        assert_eq!(
            record_names(&records),
            vec!["forward", "forward2", "reverse", "unmapped"]
        );
        assert_eq!(records.last().expect("last record").flags & 0x4, 0x4);

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn queryname_lexicographical_sort_rewrites_header_and_orders_by_name() {
        let input = write_temp_file(
            "sort-queryname-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(0, 3, "read2", 0),
                    build_light_record(0, 2, "read10", 0),
                    build_light_record(0, 1, "read1", 0),
                ],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-sort-queryname-output-{}.bam",
            std::process::id()
        ));

        let result = sort_bam(&SortExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            force: true,
            order: SortOrder::Queryname,
            queryname_suborder: Some(QuerynameSubOrder::Lexicographical),
            threads: 1,
            memory_limit: None,
        })
        .expect("sort should succeed");

        let header = parse_bam_header(&output).expect("output header should parse");
        assert_eq!(header.header.hd.sort_order.as_deref(), Some("queryname"));
        assert_eq!(
            header.header.hd.sub_sort_order.as_deref(),
            Some("queryname:lexicographical")
        );
        assert_eq!(
            result.produced_sub_order,
            Some(QuerynameSubOrder::Lexicographical)
        );
        let records = read_sorted_records(&output);
        assert_eq!(record_names(&records), vec!["read1", "read10", "read2"]);

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn external_queryname_sort_spills_and_stably_merges_bounded_runs() {
        let input = write_temp_file(
            "sort-external-queryname-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(0, 7, "read2", 0),
                    build_light_record(0, 2, "read1", 0x10),
                    build_light_record(0, 1, "read1", 0),
                    build_light_record(0, 9, "read3", 0),
                ],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-sort-external-queryname-output-{}.bam",
            std::process::id()
        ));

        let result = sort_bam(&SortExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            force: true,
            order: SortOrder::Queryname,
            queryname_suborder: Some(QuerynameSubOrder::Lexicographical),
            threads: 2,
            memory_limit: Some(1),
        })
        .expect("bounded external sort should succeed");

        let records = read_sorted_records(&output);
        assert_eq!(
            record_names(&records),
            vec!["read1", "read1", "read2", "read3"]
        );
        assert_eq!(
            records
                .iter()
                .filter(|record| record.read_name == "read1")
                .map(|record| record.pos)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(result.records_read, 4);
        assert_eq!(result.records_written, 4);
        assert!(
            result.notes[0].contains("4 temporary run(s)"),
            "one-byte budget should force one record per run"
        );
        let run_prefix = format!(
            ".{}.bamana-sort-{}.run-",
            output.file_name().unwrap().to_string_lossy(),
            std::process::id()
        );
        assert!(
            fs::read_dir(output.parent().expect("output parent"))
                .expect("output parent should list")
                .all(|entry| !entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&run_prefix)),
            "successful external sort must remove temporary runs"
        );

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn existing_output_requires_force_and_preserves_sentinel() {
        let input = write_temp_file(
            "sort-existing-output-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "read1", 0)],
            ),
        );
        let output = write_temp_file("sort-existing-output-sentinel", "bam", b"sentinel");

        let error = sort_bam(&SortExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            force: false,
            order: SortOrder::Coordinate,
            queryname_suborder: None,
            threads: 1,
            memory_limit: None,
        })
        .expect_err("existing output should require force");

        assert_eq!(error.to_json_error().code, "output_exists");
        assert_eq!(
            fs::read(&output).expect("sentinel should still exist"),
            b"sentinel"
        );

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn finalize_failure_preserves_non_file_output_and_cleans_temp() {
        let input = write_temp_file(
            "sort-output-dir-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:unsorted\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "read1", 0)],
            ),
        );
        let output =
            std::env::temp_dir().join(format!("bamana-sort-output-dir-{}.bam", std::process::id()));
        fs::create_dir_all(&output).expect("directory collision should create");
        let temp = super::temporary_output_path(&output);

        let error = sort_bam(&SortExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            force: true,
            order: SortOrder::Coordinate,
            queryname_suborder: None,
            threads: 1,
            memory_limit: None,
        })
        .expect_err("directory output should fail at finalization");

        assert_eq!(error.to_json_error().code, "write_error");
        assert!(output.is_dir());
        assert!(!temp.exists());

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_dir(output).expect("directory collision should be removable");
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

    fn read_sorted_records(path: &std::path::Path) -> Vec<crate::bam::records::RecordLayout> {
        let mut scanner = BamScanner::open(path).expect("sorted BAM should open through scanner");
        let mut records = Vec::new();
        while let Some(record) = scanner.next_record().expect("sorted record should scan") {
            records.push(record.to_record_layout());
        }
        records
    }

    fn record_names(records: &[crate::bam::records::RecordLayout]) -> Vec<&str> {
        records
            .iter()
            .map(|record| record.read_name.as_str())
            .collect()
    }
}
