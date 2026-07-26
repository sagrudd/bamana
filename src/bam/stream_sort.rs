use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    bam::{
        records::RecordLayout,
        scan::BamScanner,
        sort::{
            QuerynameSubOrder, SortOrder, compare_coordinate_layouts, compare_queryname_layouts,
        },
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
    output_safety::{finalize_completed_output, remove_stale_temp},
};

#[derive(Debug)]
pub struct StreamSortOptions {
    pub output_path: PathBuf,
    pub force: bool,
    pub order: SortOrder,
    pub queryname_suborder: Option<QuerynameSubOrder>,
    pub threads: usize,
    pub memory_limit: u64,
    pub compression_level: u32,
}

#[derive(Debug)]
pub struct StreamSortResult {
    pub records_read: u64,
    pub records_written: u64,
    pub run_count: usize,
    pub overwritten: bool,
}

#[derive(Debug)]
struct SortableRecord {
    layout: RecordLayout,
    ordinal: u64,
}

pub fn sort_record_stream<I>(
    options: &StreamSortOptions,
    header_payload: &[u8],
    records: I,
) -> Result<StreamSortResult, AppError>
where
    I: IntoIterator<Item = Result<RecordLayout, AppError>>,
{
    validate_options(options)?;
    let preexisting_output = options.output_path.exists();
    if preexisting_output && !options.force {
        return Err(AppError::OutputExists {
            path: options.output_path.clone(),
        });
    }

    let temp_path = temporary_output_path(&options.output_path);
    remove_stale_temp(&temp_path);
    let mut runs = TemporaryRuns::default();
    let mut buffered = Vec::new();
    let mut estimated_bytes = 0_u64;
    let mut records_read = 0_u64;

    for record in records {
        let layout = record?;
        estimated_bytes = estimated_bytes.saturating_add(estimated_record_bytes(&layout));
        buffered.push(SortableRecord {
            layout,
            ordinal: records_read,
        });
        records_read += 1;
        if estimated_bytes >= options.memory_limit {
            spill_run(options, header_payload, &mut buffered, &mut runs)?;
            estimated_bytes = 0;
        }
    }
    if !buffered.is_empty() {
        spill_run(options, header_payload, &mut buffered, &mut runs)?;
    }

    let records_written = merge_runs(
        &runs.paths,
        &temp_path,
        header_payload,
        options.order,
        options.queryname_suborder,
        options.threads,
        options.compression_level,
    )
    .inspect_err(|_| {
        let _ = fs::remove_file(&temp_path);
    })?;
    if records_written != records_read {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::InvalidRecord {
            path: options.output_path.clone(),
            detail: format!(
                "Stream sort read {records_read} records but merged {records_written}."
            ),
        });
    }

    finalize_completed_output(&temp_path, &options.output_path, options.force)?;
    let run_count = runs.paths.len();
    runs.remove_all();
    Ok(StreamSortResult {
        records_read,
        records_written,
        run_count,
        overwritten: preexisting_output && options.force,
    })
}

fn validate_options(options: &StreamSortOptions) -> Result<(), AppError> {
    if options.memory_limit == 0 {
        return Err(AppError::InvalidSortRequest {
            path: options.output_path.clone(),
            detail: "Stream-sort memory limit must be greater than zero.".to_string(),
        });
    }
    if options.compression_level > 9 {
        return Err(AppError::InvalidSortRequest {
            path: options.output_path.clone(),
            detail: "BGZF compression level must be between zero and nine.".to_string(),
        });
    }
    if matches!(options.queryname_suborder, Some(QuerynameSubOrder::Natural)) {
        return Err(AppError::Unimplemented {
            path: options.output_path.clone(),
            detail: "Natural queryname stream sorting is not implemented.".to_string(),
        });
    }
    Ok(())
}

fn sort_records(
    records: &mut [SortableRecord],
    options: &StreamSortOptions,
) -> Result<(), AppError> {
    let order = options.order;
    let suborder = options.queryname_suborder;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.threads.max(1))
        .build()
        .map_err(|error| AppError::InvalidSortRequest {
            path: options.output_path.clone(),
            detail: format!("Cannot create stream-sort worker pool: {error}"),
        })?;
    pool.install(|| {
        use rayon::prelude::*;
        records.par_sort_by(|left, right| {
            compare(
                &left.layout,
                left.ordinal,
                &right.layout,
                right.ordinal,
                order,
                suborder,
            )
        });
    });
    Ok(())
}

fn spill_run(
    options: &StreamSortOptions,
    header_payload: &[u8],
    records: &mut Vec<SortableRecord>,
    runs: &mut TemporaryRuns,
) -> Result<(), AppError> {
    sort_records(records, options)?;
    let path = temporary_run_path(&options.output_path, runs.paths.len());
    remove_stale_temp(&path);
    let write_result = (|| {
        let mut writer = BgzfWriter::create_with_threads_and_level(
            &path,
            options.threads,
            options.compression_level,
        )?;
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
    paths: &[PathBuf],
    output: &Path,
    header_payload: &[u8],
    order: SortOrder,
    suborder: Option<QuerynameSubOrder>,
    threads: usize,
    compression_level: u32,
) -> Result<u64, AppError> {
    let mut scanners = paths
        .iter()
        .map(|path| BamScanner::open(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut heap = BinaryHeap::new();
    for (run_index, scanner) in scanners.iter_mut().enumerate() {
        if let Some(record) = scanner.next_record()? {
            heap.push(MergeItem::new(
                record.to_record_layout(),
                run_index,
                0,
                order,
                suborder,
            ));
        }
    }
    let result = (|| {
        let mut writer =
            BgzfWriter::create_with_threads_and_level(output, threads, compression_level)?;
        writer.write_all(header_payload)?;
        let mut written = 0_u64;
        while let Some(item) = heap.pop() {
            writer.write_all(&serialize_record_layout(&item.layout))?;
            written += 1;
            if let Some(record) = scanners[item.run_index].next_record()? {
                heap.push(MergeItem::new(
                    record.to_record_layout(),
                    item.run_index,
                    item.sequence + 1,
                    order,
                    suborder,
                ));
            }
        }
        writer.finish()?;
        Ok(written)
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}

struct MergeItem {
    layout: RecordLayout,
    run_index: usize,
    sequence: u64,
    order: SortOrder,
    suborder: Option<QuerynameSubOrder>,
}

impl MergeItem {
    fn new(
        layout: RecordLayout,
        run_index: usize,
        sequence: u64,
        order: SortOrder,
        suborder: Option<QuerynameSubOrder>,
    ) -> Self {
        Self {
            layout,
            run_index,
            sequence,
            order,
            suborder,
        }
    }
}

impl Eq for MergeItem {}

impl PartialEq for MergeItem {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd for MergeItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeItem {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(&other.layout, 0, &self.layout, 0, self.order, self.suborder)
            .then_with(|| other.run_index.cmp(&self.run_index))
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

fn compare(
    left: &RecordLayout,
    left_ordinal: u64,
    right: &RecordLayout,
    right_ordinal: u64,
    order: SortOrder,
    _suborder: Option<QuerynameSubOrder>,
) -> Ordering {
    match order {
        SortOrder::Coordinate => {
            compare_coordinate_layouts(left, left_ordinal, right, right_ordinal)
        }
        SortOrder::Queryname => compare_queryname_layouts(left, left_ordinal, right, right_ordinal),
    }
}

fn estimated_record_bytes(layout: &RecordLayout) -> u64 {
    (layout.block_size as u64)
        .saturating_add(4)
        .saturating_add(128)
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    output.with_file_name(format!(
        ".{name}.bamana-stream-sort-{}.tmp",
        std::process::id()
    ))
}

fn temporary_run_path(output: &Path, index: usize) -> PathBuf {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    output.with_file_name(format!(
        ".{name}.bamana-stream-sort-{}.run-{index:06}.bam",
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
