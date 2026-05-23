use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
};

use flate2::{Compression, GzBuilder};
use serde::Serialize;

use crate::{
    bam::{
        header::serialize_bam_header_payload,
        index::IndexKind,
        scan::BamScanner,
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
    fastq::{
        FastqRecord, open_fastq_reader_with_label, read_next_fastq_record, resolved_threads,
        write_fastq_record_to,
    },
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    ingest::sam::count_sam_records,
    json::CommandResponse,
};

use crate::fastq::gzi::{
    FastqGziPlannedRange, ensure_fastq_gzi, fastq_gzi_output_path, plan_fastq_gzi_ranges,
};

const TARGET_FASTQ_BATCH_RECORDS: usize = 4096;

#[derive(Debug)]
pub struct ExplodeRequest {
    pub input: PathBuf,
    pub out_dir: PathBuf,
    pub explode: usize,
    pub threads: usize,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct ExplodePayload {
    pub format: String,
    pub input: ExplodeInputInfo,
    pub explode: ExplodePlanInfo,
    pub outputs: Vec<ExplodeOutputInfo>,
    pub index: ExplodeIndexInfo,
    pub checksum_verification: ExplodeChecksumInfo,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplodeInputInfo {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ExplodePlanInfo {
    pub requested_parts: usize,
    pub produced_parts: usize,
    pub strategy: String,
    pub threads_used: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoints_available: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ExplodeOutputInfo {
    pub path: String,
    pub records_written: u64,
    pub record_start: u64,
    pub record_end: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approx_compressed_start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approx_compressed_end: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ExplodeIndexInfo {
    pub requested: bool,
    pub created: bool,
    pub kind: IndexKind,
}

#[derive(Debug, Serialize)]
pub struct ExplodeChecksumInfo {
    pub requested: bool,
    pub performed: bool,
    pub mode: Option<String>,
    pub input_digest: Option<String>,
    pub output_digest: Option<String>,
    pub r#match: Option<bool>,
}

#[derive(Debug)]
struct FastqBatchJob {
    shard_index: usize,
    batch_index: usize,
    records: Vec<FastqRecord>,
}

#[derive(Debug)]
struct FastqBatchResult {
    shard_index: usize,
    batch_index: usize,
    record_count: u64,
    compressed: Result<Vec<u8>, AppError>,
}

pub fn run(request: ExplodeRequest) -> CommandResponse<ExplodePayload> {
    let input = request.input.clone();
    match run_impl(&request) {
        Ok(payload) => CommandResponse::success("explode", Some(input.as_path()), payload),
        Err(error) => CommandResponse::failure("explode", Some(input.as_path()), error),
    }
}

fn run_impl(request: &ExplodeRequest) -> Result<ExplodePayload, AppError> {
    let probe = probe_path(&request.input)?;
    let format = probe.detected_format.to_string();
    let mut payload = ExplodePayload {
        format: format.clone(),
        input: ExplodeInputInfo {
            path: request.input.to_string_lossy().into_owned(),
        },
        explode: ExplodePlanInfo {
            requested_parts: request.explode,
            produced_parts: 0,
            strategy: "unresolved".to_string(),
            threads_used: 0,
            total_records: None,
            checkpoints_available: None,
        },
        outputs: Vec::new(),
        index: ExplodeIndexInfo {
            requested: false,
            created: false,
            kind: IndexKind::Unknown,
        },
        checksum_verification: ExplodeChecksumInfo {
            requested: false,
            performed: false,
            mode: None,
            input_digest: None,
            output_digest: None,
            r#match: None,
        },
        notes: Vec::new(),
    };

    if request.explode == 0 {
        return Err(AppError::InvalidTargetRecords {
            path: request.input.clone(),
            detail: "--explode must be at least 1.".to_string(),
        });
    }
    if request.explode >= 1_000 {
        return Err(AppError::InvalidTargetRecords {
            path: request.input.clone(),
            detail: "--explode must be less than 1000.".to_string(),
        });
    }

    let (outputs, notes, total_records, checkpoints_available, threads_used, index_info) =
        match probe.detected_format {
            DetectedFormat::FastqGz => {
                let execution = explode_fastq_gz(request)?;
                (
                    execution.outputs,
                    execution.notes,
                    Some(execution.total_records),
                    Some(execution.checkpoints_available),
                    execution.threads_used,
                    ExplodeIndexInfo {
                        requested: true,
                        created: execution.index_created,
                        kind: IndexKind::Gzi,
                    },
                )
            }
            DetectedFormat::Bam => {
                if probe.container != ContainerKind::Bgzf {
                    return Err(AppError::InvalidBam {
                        path: request.input.clone(),
                        detail: "Input did not present a BGZF-compatible container header."
                            .to_string(),
                    });
                }
                let execution = explode_bam(request)?;
                (
                    execution.outputs,
                    execution.notes,
                    Some(execution.total_records),
                    None,
                    execution.threads_used,
                    ExplodeIndexInfo {
                        requested: false,
                        created: false,
                        kind: IndexKind::Unknown,
                    },
                )
            }
            DetectedFormat::Sam => {
                let execution = explode_sam(request)?;
                (
                    execution.outputs,
                    execution.notes,
                    Some(execution.total_records),
                    None,
                    execution.threads_used,
                    ExplodeIndexInfo {
                        requested: false,
                        created: false,
                        kind: IndexKind::Unknown,
                    },
                )
            }
            other => {
                return Err(AppError::UnsupportedFormat {
                    path: request.input.clone(),
                    format: format!(
                        "Explode currently supports BAM, SAM, and FASTQ.GZ inputs; detected {other}."
                    ),
                });
            }
        };

    payload.explode.produced_parts = outputs.len();
    payload.explode.strategy = if probe.detected_format == DetectedFormat::FastqGz {
        "indexed_record_ranges".to_string()
    } else {
        "contiguous_record_ranges".to_string()
    };
    payload.explode.threads_used = threads_used;
    payload.explode.total_records = total_records;
    payload.explode.checkpoints_available = checkpoints_available;
    payload.outputs = outputs;
    payload.index = index_info;
    payload.notes = notes;
    Ok(payload)
}

#[derive(Debug)]
struct ExplodeExecution {
    outputs: Vec<ExplodeOutputInfo>,
    notes: Vec<String>,
    total_records: u64,
    checkpoints_available: usize,
    threads_used: usize,
    index_created: bool,
}

#[derive(Debug)]
struct SerialExplodeExecution {
    outputs: Vec<ExplodeOutputInfo>,
    notes: Vec<String>,
    total_records: u64,
    threads_used: usize,
}

fn explode_fastq_gz(request: &ExplodeRequest) -> Result<ExplodeExecution, AppError> {
    let index_path = fastq_gzi_output_path(&request.input);
    let index_created = !index_path.is_file();
    let index = ensure_fastq_gzi(&request.input)?;
    let planned_ranges =
        plan_ranges_for_request(&request.input, &index, request.explode, request.force)?;
    let output_paths = build_output_paths(
        &request.input,
        &request.out_dir,
        ".fastq.gz",
        request.explode,
    );
    prepare_output_targets(&request.out_dir, &output_paths, request.force)?;
    let temp_paths = output_paths
        .iter()
        .map(|path| temporary_output_path(path, "explode"))
        .collect::<Vec<_>>();

    let resolved = resolved_threads(request.threads);
    let worker_count = resolved.saturating_sub(1).max(1);
    let batch_records = batch_record_target(index.total_records, request.explode, worker_count);

    let (job_tx, job_rx) = mpsc::channel::<FastqBatchJob>();
    let (result_tx, result_rx) = mpsc::channel::<FastqBatchResult>();
    let shared_rx = Arc::new(Mutex::new(job_rx));
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let job_rx = Arc::clone(&shared_rx);
        let result_tx = result_tx.clone();
        let label = request.input.clone();
        handles.push(thread::spawn(move || {
            while let Some(job) = recv_fastq_job(&job_rx) {
                let record_count = job.records.len() as u64;
                let compressed = compress_fastq_batch(&job.records, &label);
                if result_tx
                    .send(FastqBatchResult {
                        shard_index: job.shard_index,
                        batch_index: job.batch_index,
                        record_count,
                        compressed,
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
    drop(result_tx);

    let write_result = (|| -> Result<Vec<u64>, AppError> {
        let mut pending = (0..request.explode)
            .map(|_| BTreeMap::<usize, (u64, Result<Vec<u8>, AppError>)>::new())
            .collect::<Vec<_>>();
        let mut next_batch_to_write = vec![0_usize; request.explode];
        let mut written_records = vec![0_u64; request.explode];
        let mut submitted_batches = vec![0_usize; request.explode];
        let mut writers = (0..request.explode)
            .map(|_| None::<BufWriter<File>>)
            .collect::<Vec<_>>();

        let mut reader = open_fastq_reader_with_label(&request.input, &request.input)?;
        let mut shard_index = 0_usize;
        let mut batch = Vec::with_capacity(batch_records);
        let mut global_record_index = 0_u64;

        loop {
            let Some(record) = read_next_fastq_record(&mut reader, &request.input)? else {
                break;
            };

            batch.push(record);
            global_record_index += 1;
            let shard_end = planned_ranges[shard_index].record_end;
            let reached_shard_end = global_record_index == shard_end;

            if batch.len() >= batch_records || reached_shard_end {
                submit_fastq_batch(
                    &job_tx,
                    shard_index,
                    &mut batch,
                    &mut submitted_batches[shard_index],
                )?;
                drain_fastq_results(
                    &result_rx,
                    &mut pending,
                    &mut writers,
                    &temp_paths,
                    &mut next_batch_to_write,
                    &mut written_records,
                )?;
            }

            if reached_shard_end && shard_index + 1 < request.explode {
                shard_index += 1;
            }
        }

        if !batch.is_empty() {
            submit_fastq_batch(
                &job_tx,
                shard_index,
                &mut batch,
                &mut submitted_batches[shard_index],
            )?;
        }
        drop(job_tx);

        let total_batches = submitted_batches.iter().sum::<usize>();
        let mut written_batches = next_batch_to_write.iter().sum::<usize>();
        while written_batches < total_batches {
            let result = result_rx.recv().map_err(|_| AppError::Internal {
                message: "explode worker channel closed before all FASTQ.GZ batches completed."
                    .to_string(),
            })?;
            pending[result.shard_index]
                .insert(result.batch_index, (result.record_count, result.compressed));
            write_ready_fastq_batches(
                &mut pending,
                &mut writers,
                &temp_paths,
                &mut next_batch_to_write,
                &mut written_records,
            )?;
            written_batches = next_batch_to_write.iter().sum::<usize>();
        }

        for writer in writers.into_iter().flatten() {
            let mut writer = writer;
            writer.flush().map_err(|error| AppError::WriteError {
                path: request.out_dir.clone(),
                message: error.to_string(),
            })?;
        }

        Ok(written_records)
    })();

    let written_records = match write_result {
        Ok(written_records) => written_records,
        Err(error) => {
            cleanup_temp_outputs(&temp_paths);
            return Err(error);
        }
    };

    for handle in handles {
        handle.join().map_err(|_| AppError::Internal {
            message: "explode FASTQ.GZ worker thread panicked.".to_string(),
        })?;
    }

    finalize_output_targets(&temp_paths, &output_paths, request.force)?;

    let outputs = planned_ranges
        .iter()
        .enumerate()
        .map(|(index, range)| ExplodeOutputInfo {
            path: output_paths[index].to_string_lossy().into_owned(),
            records_written: written_records[index],
            record_start: range.record_start,
            record_end: range.record_end,
            approx_compressed_start: Some(range.approx_compressed_start),
            approx_compressed_end: Some(range.approx_compressed_end),
        })
        .collect::<Vec<_>>();

    let mut notes = vec![
        "FASTQ.GZ explode used FASTQ.GZI planning metadata to derive contiguous checkpoint-aligned record ranges without a pre-enumeration pass."
            .to_string(),
        "Each FASTQ.GZ shard is written as a concatenated gzip-member stream so parsing and compression can overlap."
            .to_string(),
        "Shard boundaries follow available FASTQ.GZI cutpoints and therefore may vary in size; every read remains in exactly one shard and encounter order is preserved within each shard."
            .to_string(),
    ];
    if index_created {
        notes.push(
            "FASTQ.GZI was created automatically before exploding the FASTQ.GZ input.".to_string(),
        );
    } else {
        notes.push("Existing FASTQ.GZI metadata was reused for explode planning.".to_string());
    }
    notes.push(format!(
        "FASTQ.GZI planning used {} dense checkpoints across the indexed input.",
        index.checkpoints.len()
    ));

    Ok(ExplodeExecution {
        outputs,
        notes,
        total_records: index.total_records,
        checkpoints_available: index.checkpoints.len(),
        threads_used: resolved,
        index_created,
    })
}

fn explode_bam(request: &ExplodeRequest) -> Result<SerialExplodeExecution, AppError> {
    let mut count_scanner = BamScanner::open(&request.input)?;
    let header = count_scanner.header().clone();
    let total_records = count_bam_records(&mut count_scanner, &request.input)?;
    validate_explode_parts(&request.input, total_records, request.explode)?;
    let planned_ranges = plan_serial_ranges(total_records, request.explode);
    let output_paths =
        build_output_paths(&request.input, &request.out_dir, ".bam", request.explode);
    prepare_output_targets(&request.out_dir, &output_paths, request.force)?;
    let temp_paths = output_paths
        .iter()
        .map(|path| temporary_output_path(path, "explode"))
        .collect::<Vec<_>>();

    let header_payload = serialize_bam_header_payload(
        &request.input,
        &header.header.raw_header_text,
        &header.header.references,
    )?;
    let write_result = (|| -> Result<Vec<u64>, AppError> {
        let mut scanner = BamScanner::open(&request.input)?;
        let mut writers = create_bam_writers(&temp_paths, &header_payload)?;
        let mut written_records = vec![0_u64; request.explode];
        let mut current_shard = 0_usize;
        let mut global_record_index = 0_u64;

        while let Some(record) = scanner.next_record()? {
            let record = record.to_record_layout();
            writers[current_shard].write_all(&serialize_record_layout(&record))?;
            written_records[current_shard] += 1;
            global_record_index += 1;
            if global_record_index == planned_ranges[current_shard].record_end
                && current_shard + 1 < request.explode
            {
                current_shard += 1;
            }
        }

        if global_record_index != scanner.records_read() {
            return Err(AppError::InvalidRecord {
                path: request.input.clone(),
                detail:
                    "Explode scanner record count diverged from records materialized for sharding."
                        .to_string(),
            });
        }

        for writer in writers {
            writer.finish()?;
        }

        Ok(written_records)
    })();

    let written_records = match write_result {
        Ok(written_records) => written_records,
        Err(error) => {
            cleanup_temp_outputs(&temp_paths);
            return Err(error);
        }
    };

    finalize_output_targets(&temp_paths, &output_paths, request.force)?;

    Ok(SerialExplodeExecution {
        outputs: planned_ranges
            .iter()
            .enumerate()
            .map(|(index, range)| ExplodeOutputInfo {
                path: output_paths[index].to_string_lossy().into_owned(),
                records_written: written_records[index],
                record_start: range.record_start,
                record_end: range.record_end,
                approx_compressed_start: None,
                approx_compressed_end: None,
            })
            .collect(),
        notes: vec![
            "BAM explode preserved the original BAM header in every shard and split records by contiguous encounter-order ranges."
                .to_string(),
            "BAM shard writing uses BamScanner-owned records bridged into Bamana's native BGZF writer path."
                .to_string(),
        ],
        total_records,
        threads_used: 1,
    })
}

fn explode_sam(request: &ExplodeRequest) -> Result<SerialExplodeExecution, AppError> {
    let total_records = count_sam_records(&request.input)?;
    validate_explode_parts(&request.input, total_records, request.explode)?;
    let planned_ranges = plan_serial_ranges(total_records, request.explode);
    let output_paths =
        build_output_paths(&request.input, &request.out_dir, ".sam", request.explode);
    prepare_output_targets(&request.out_dir, &output_paths, request.force)?;
    let temp_paths = output_paths
        .iter()
        .map(|path| temporary_output_path(path, "explode"))
        .collect::<Vec<_>>();
    let header_text = read_sam_header_text(&request.input)?;

    let write_result = (|| -> Result<Vec<u64>, AppError> {
        let mut writers = temp_paths
            .iter()
            .map(|path| {
                let file = File::create(path).map_err(|error| AppError::WriteError {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
                let mut writer = BufWriter::new(file);
                if !header_text.is_empty() {
                    writer.write_all(header_text.as_bytes()).map_err(|error| {
                        AppError::WriteError {
                            path: path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                }
                Ok(writer)
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let mut written_records = vec![0_u64; request.explode];
        let mut current_shard = 0_usize;
        let mut global_record_index = 0_u64;

        let file =
            File::open(&request.input).map_err(|error| AppError::from_io(&request.input, error))?;
        let reader = BufReader::new(file);
        for line_result in reader.lines() {
            let line = line_result.map_err(|error| AppError::from_io(&request.input, error))?;
            if line.starts_with('@') || line.is_empty() {
                continue;
            }

            writers[current_shard]
                .write_all(line.as_bytes())
                .and_then(|()| writers[current_shard].write_all(b"\n"))
                .map_err(|error| AppError::WriteError {
                    path: temp_paths[current_shard].clone(),
                    message: error.to_string(),
                })?;
            written_records[current_shard] += 1;
            global_record_index += 1;
            if global_record_index == planned_ranges[current_shard].record_end
                && current_shard + 1 < request.explode
            {
                current_shard += 1;
            }
        }

        for mut writer in writers {
            writer.flush().map_err(|error| AppError::WriteError {
                path: request.out_dir.clone(),
                message: error.to_string(),
            })?;
        }

        Ok(written_records)
    })();

    let written_records = match write_result {
        Ok(written_records) => written_records,
        Err(error) => {
            cleanup_temp_outputs(&temp_paths);
            return Err(error);
        }
    };

    finalize_output_targets(&temp_paths, &output_paths, request.force)?;

    Ok(SerialExplodeExecution {
        outputs: planned_ranges
            .iter()
            .enumerate()
            .map(|(index, range)| ExplodeOutputInfo {
                path: output_paths[index].to_string_lossy().into_owned(),
                records_written: written_records[index],
                record_start: range.record_start,
                record_end: range.record_end,
                approx_compressed_start: None,
                approx_compressed_end: None,
            })
            .collect(),
        notes: vec![
            "SAM explode preserved the original SAM header in every shard and split alignment lines by contiguous encounter-order ranges."
                .to_string(),
        ],
        total_records,
        threads_used: 1,
    })
}

fn create_bam_writers(
    paths: &[PathBuf],
    header_payload: &[u8],
) -> Result<Vec<BgzfWriter>, AppError> {
    let mut writers = Vec::with_capacity(paths.len());
    for path in paths {
        let mut writer = BgzfWriter::create(path)?;
        writer.write_all(header_payload)?;
        writers.push(writer);
    }
    Ok(writers)
}

fn count_bam_records(scanner: &mut BamScanner, path: &Path) -> Result<u64, AppError> {
    let mut records = 0_u64;
    while scanner.next_record()?.is_some() {
        records += 1;
    }
    if records != scanner.records_read() {
        return Err(AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "Explode scanner record count diverged while counting BAM records.".to_string(),
        });
    }
    Ok(records)
}

fn validate_explode_parts(path: &Path, total_records: u64, parts: usize) -> Result<(), AppError> {
    if total_records == 0 {
        return Err(AppError::InvalidTargetRecords {
            path: path.to_path_buf(),
            detail: "Input did not contain any records to explode.".to_string(),
        });
    }
    if parts as u64 > total_records {
        return Err(AppError::InvalidTargetRecords {
            path: path.to_path_buf(),
            detail: format!(
                "Requested {parts} output shards, but the input only contains {total_records} records."
            ),
        });
    }
    Ok(())
}

fn build_output_paths(input: &Path, out_dir: &Path, extension: &str, parts: usize) -> Vec<PathBuf> {
    let stem = input_stem(input);
    (0..parts)
        .map(|index| out_dir.join(format!("{stem}.part{:04}{extension}", index + 1)))
        .collect()
}

fn input_stem(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("bamana-explode");
    for suffix in [".fastq.gz", ".fq.gz", ".bam", ".sam", ".gz"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            return stripped.to_string();
        }
    }
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name)
        .to_string()
}

fn temporary_output_path(final_path: &Path, label: &str) -> PathBuf {
    let stem = final_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-explode-output");
    final_path.with_file_name(format!(".{stem}.bamana-{label}-{}.tmp", std::process::id()))
}

fn prepare_output_targets(
    out_dir: &Path,
    output_paths: &[PathBuf],
    force: bool,
) -> Result<(), AppError> {
    if out_dir.exists() && !out_dir.is_dir() {
        return Err(AppError::WriteError {
            path: out_dir.to_path_buf(),
            message: "Output directory path existed but was not a directory.".to_string(),
        });
    }
    fs::create_dir_all(out_dir).map_err(|error| AppError::WriteError {
        path: out_dir.to_path_buf(),
        message: error.to_string(),
    })?;

    for path in output_paths {
        if path.exists() && !force {
            return Err(AppError::OutputExists { path: path.clone() });
        }
    }

    Ok(())
}

fn finalize_output_targets(
    temp_paths: &[PathBuf],
    output_paths: &[PathBuf],
    force: bool,
) -> Result<(), AppError> {
    for (temp_path, output_path) in temp_paths.iter().zip(output_paths.iter()) {
        if output_path.exists() && force {
            fs::remove_file(output_path).map_err(|error| AppError::WriteError {
                path: output_path.clone(),
                message: error.to_string(),
            })?;
        }
        fs::rename(temp_path, output_path).map_err(|error| AppError::WriteError {
            path: output_path.clone(),
            message: error.to_string(),
        })?;
    }
    Ok(())
}

fn cleanup_temp_outputs(temp_paths: &[PathBuf]) {
    for path in temp_paths {
        let _ = fs::remove_file(path);
    }
}

fn plan_serial_ranges(total_records: u64, parts: usize) -> Vec<FastqGziPlannedRange> {
    (0..parts)
        .map(|part_index| FastqGziPlannedRange {
            part_index,
            record_start: (total_records * part_index as u64) / parts as u64,
            record_end: (total_records * (part_index as u64 + 1)) / parts as u64,
            approx_compressed_start: 0,
            approx_compressed_end: 0,
            approx_uncompressed_start: 0,
            approx_uncompressed_end: 0,
        })
        .collect()
}

fn plan_ranges_for_request(
    path: &Path,
    index: &crate::fastq::gzi::FastqGziIndexSummary,
    parts: usize,
    _force: bool,
) -> Result<Vec<FastqGziPlannedRange>, AppError> {
    plan_fastq_gzi_ranges(index, parts).map_err(|detail| AppError::InvalidTargetRecords {
        path: path.to_path_buf(),
        detail,
    })
}

fn batch_record_target(total_records: u64, parts: usize, worker_count: usize) -> usize {
    let records_per_shard = (total_records / parts.max(1) as u64).max(1);
    let records_per_batch = (records_per_shard / worker_count.max(1) as u64).max(1);
    records_per_batch.clamp(1024, TARGET_FASTQ_BATCH_RECORDS as u64) as usize
}

fn recv_fastq_job(job_rx: &Arc<Mutex<mpsc::Receiver<FastqBatchJob>>>) -> Option<FastqBatchJob> {
    job_rx.lock().ok()?.recv().ok()
}

fn submit_fastq_batch(
    job_tx: &mpsc::Sender<FastqBatchJob>,
    shard_index: usize,
    batch: &mut Vec<FastqRecord>,
    next_batch_index: &mut usize,
) -> Result<(), AppError> {
    let records = std::mem::take(batch);
    job_tx
        .send(FastqBatchJob {
            shard_index,
            batch_index: *next_batch_index,
            records,
        })
        .map_err(|_| AppError::Internal {
            message: "explode FASTQ.GZ worker queue closed unexpectedly.".to_string(),
        })?;
    *next_batch_index += 1;
    Ok(())
}

fn drain_fastq_results(
    result_rx: &mpsc::Receiver<FastqBatchResult>,
    pending: &mut [BTreeMap<usize, (u64, Result<Vec<u8>, AppError>)>],
    writers: &mut [Option<BufWriter<File>>],
    temp_paths: &[PathBuf],
    next_batch_to_write: &mut [usize],
    written_records: &mut [u64],
) -> Result<(), AppError> {
    while let Ok(result) = result_rx.try_recv() {
        pending[result.shard_index]
            .insert(result.batch_index, (result.record_count, result.compressed));
    }
    write_ready_fastq_batches(
        pending,
        writers,
        temp_paths,
        next_batch_to_write,
        written_records,
    )
}

fn write_ready_fastq_batches(
    pending: &mut [BTreeMap<usize, (u64, Result<Vec<u8>, AppError>)>],
    writers: &mut [Option<BufWriter<File>>],
    temp_paths: &[PathBuf],
    next_batch_to_write: &mut [usize],
    written_records: &mut [u64],
) -> Result<(), AppError> {
    for shard_index in 0..pending.len() {
        while let Some((record_count, compressed)) =
            pending[shard_index].remove(&next_batch_to_write[shard_index])
        {
            let compressed = compressed?;
            if writers[shard_index].is_none() {
                let file = File::create(&temp_paths[shard_index]).map_err(|error| {
                    AppError::WriteError {
                        path: temp_paths[shard_index].clone(),
                        message: error.to_string(),
                    }
                })?;
                writers[shard_index] = Some(BufWriter::new(file));
            }
            let writer = writers[shard_index].as_mut().expect("writer should exist");
            writer
                .write_all(&compressed)
                .map_err(|error| AppError::WriteError {
                    path: temp_paths[shard_index].clone(),
                    message: error.to_string(),
                })?;
            written_records[shard_index] += record_count;
            next_batch_to_write[shard_index] += 1;
        }
    }
    Ok(())
}

fn compress_fastq_batch(records: &[FastqRecord], path: &Path) -> Result<Vec<u8>, AppError> {
    let mut payload = Vec::new();
    for record in records {
        write_fastq_record_to(&mut payload, record, path)?;
    }

    let mut encoder = GzBuilder::new().write(Vec::new(), Compression::fast());
    encoder
        .write_all(&payload)
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    encoder.finish().map_err(|error| AppError::WriteError {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn read_sam_header_text(path: &Path) -> Result<String, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let reader = BufReader::new(file);
    let mut header_lines = Vec::new();
    for line_result in reader.lines() {
        let line = line_result.map_err(|error| AppError::from_io(path, error))?;
        if line.starts_with('@') {
            header_lines.push(line);
        } else if !line.is_empty() {
            break;
        }
    }

    if header_lines.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", header_lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    use flate2::{Compression, write::GzEncoder};

    use crate::{
        bam::{header::parse_bam_header, index::IndexKind, scan::BamScanner},
        commands::explode::{ExplodeRequest, run_impl},
        fastq::{count_fastq_records, open_fastq_reader, read_next_fastq_record},
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    #[test]
    fn explodes_fastq_gz_into_shards() {
        let input = std::env::temp_dir().join(format!(
            "bamana-explode-fastq-gz-{}.fastq.gz",
            std::process::id()
        ));
        let out_dir = std::env::temp_dir().join(format!(
            "bamana-explode-fastq-gz-out-{}",
            std::process::id()
        ));
        let file = File::create(&input).expect("fixture should create");
        let mut encoder = GzEncoder::new(file, Compression::default());
        for index in 0..12 {
            writeln!(encoder, "@read{index}").expect("header should write");
            writeln!(encoder, "ACGT").expect("sequence should write");
            writeln!(encoder, "+").expect("plus should write");
            writeln!(encoder, "!!!!").expect("quality should write");
        }
        encoder.finish().expect("gzip should finish");

        let payload = run_impl(&ExplodeRequest {
            input: input.clone(),
            out_dir: out_dir.clone(),
            explode: 3,
            threads: 0,
            force: true,
        })
        .expect("explode should succeed");

        assert_eq!(payload.format, "FASTQ.GZ");
        assert_eq!(payload.outputs.len(), 3);
        assert!(payload.index.kind == IndexKind::Gzi);
        assert_eq!(
            payload
                .outputs
                .iter()
                .map(|output| output.records_written)
                .sum::<u64>(),
            12
        );
        for output in &payload.outputs {
            let path = Path::new(&output.path);
            let count = count_fastq_records(path).expect("shard should count through reader");
            let mut reader = open_fastq_reader(path).expect("shard should open through reader");
            let first_record = read_next_fastq_record(&mut reader, path)
                .expect("shard should parse through reader")
                .expect("shard should contain records");
            assert_eq!(count, output.records_written);
            assert!(first_record.raw_header_line.starts_with("@read"));
            assert_eq!(first_record.sequence, "ACGT");
        }

        fs::remove_file(&input).expect("fixture should remove");
        for output in &payload.outputs {
            fs::remove_file(&output.path).expect("shard should remove");
        }
        fs::remove_file(crate::fastq::gzi::fastq_gzi_output_path(&input))
            .expect("index should remove");
        fs::remove_dir(&out_dir).expect("directory should remove");
    }

    #[test]
    fn explodes_bam_into_bam_shards() {
        let input = write_temp_file(
            "explode-bam-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(0, 1, "read1", 0),
                    build_light_record(0, 2, "read2", 0),
                    build_light_record(0, 3, "read3", 0),
                    build_light_record(0, 4, "read4", 0),
                ],
            ),
        );
        let out_dir =
            std::env::temp_dir().join(format!("bamana-explode-bam-out-{}", std::process::id()));

        let payload = run_impl(&ExplodeRequest {
            input: input.clone(),
            out_dir: out_dir.clone(),
            explode: 2,
            threads: 0,
            force: true,
        })
        .expect("explode should succeed");

        assert_eq!(payload.outputs.len(), 2);
        let reparsed = parse_bam_header(Path::new(&payload.outputs[0].path))
            .expect("first shard should parse");
        assert_eq!(reparsed.header.references.len(), 1);
        assert_eq!(
            read_bam_names(&payload.outputs[0].path),
            vec!["read1", "read2"]
        );
        assert_eq!(
            read_bam_names(&payload.outputs[1].path),
            vec!["read3", "read4"]
        );
        assert_eq!(payload.outputs[0].record_start, 0);
        assert_eq!(payload.outputs[0].record_end, 2);
        assert_eq!(payload.outputs[1].record_start, 2);
        assert_eq!(payload.outputs[1].record_end, 4);
        assert!(
            payload
                .notes
                .iter()
                .any(|note| note.contains("BamScanner-owned records"))
        );

        fs::remove_file(&input).expect("fixture should remove");
        for output in &payload.outputs {
            fs::remove_file(&output.path).expect("shard should remove");
        }
        fs::remove_dir(&out_dir).expect("directory should remove");
    }

    #[test]
    fn rejects_empty_bam_before_writing_shards() {
        let input = write_temp_file(
            "explode-empty-bam",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[],
            ),
        );
        let out_dir = std::env::temp_dir().join(format!(
            "bamana-explode-empty-bam-out-{}",
            std::process::id()
        ));

        let error = run_impl(&ExplodeRequest {
            input: input.clone(),
            out_dir: out_dir.clone(),
            explode: 2,
            threads: 0,
            force: true,
        })
        .expect_err("empty BAM should not explode");

        assert_eq!(error.to_json_error().code, "invalid_target_records");
        assert!(!out_dir.exists());

        fs::remove_file(&input).expect("fixture should remove");
    }

    #[test]
    fn rejects_more_bam_shards_than_records() {
        let input = write_temp_file(
            "explode-too-many-bam",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[build_light_record(0, 1, "read1", 0)],
            ),
        );
        let out_dir = std::env::temp_dir().join(format!(
            "bamana-explode-too-many-bam-out-{}",
            std::process::id()
        ));

        let error = run_impl(&ExplodeRequest {
            input: input.clone(),
            out_dir: out_dir.clone(),
            explode: 2,
            threads: 0,
            force: true,
        })
        .expect_err("too many shards should fail");

        assert_eq!(error.to_json_error().code, "invalid_target_records");
        assert!(!out_dir.exists());

        fs::remove_file(&input).expect("fixture should remove");
    }

    #[test]
    fn output_collision_requires_force() {
        let input = write_temp_file(
            "explode-collision-bam",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(0, 1, "read1", 0),
                    build_light_record(0, 2, "read2", 0),
                ],
            ),
        );
        let out_dir = std::env::temp_dir().join(format!(
            "bamana-explode-collision-bam-out-{}",
            std::process::id()
        ));
        fs::create_dir_all(&out_dir).expect("output dir should create");
        let collision = out_dir.join(format!(
            "{}.part0001.bam",
            input.file_stem().unwrap().to_string_lossy()
        ));
        fs::write(&collision, b"sentinel").expect("sentinel should write");

        let error = run_impl(&ExplodeRequest {
            input: input.clone(),
            out_dir: out_dir.clone(),
            explode: 2,
            threads: 0,
            force: false,
        })
        .expect_err("collision should fail without force");

        assert_eq!(error.to_json_error().code, "output_exists");
        assert_eq!(
            fs::read(&collision).expect("sentinel should remain"),
            b"sentinel"
        );

        fs::remove_file(&input).expect("fixture should remove");
        fs::remove_file(&collision).expect("sentinel should remove");
        fs::remove_dir(&out_dir).expect("directory should remove");
    }

    #[test]
    fn explodes_sam_into_sam_shards() {
        let input =
            std::env::temp_dir().join(format!("bamana-explode-sam-{}.sam", std::process::id()));
        let out_dir =
            std::env::temp_dir().join(format!("bamana-explode-sam-out-{}", std::process::id()));
        fs::write(
            &input,
            "@HD\tVN:1.6\tSO:unknown\n@SQ\tSN:chr1\tLN:10\nread1\t4\t*\t0\t0\t*\t*\t0\t0\tAC\t!!\nread2\t4\t*\t0\t0\t*\t*\t0\t0\tGT\t##\nread3\t4\t*\t0\t0\t*\t*\t0\t0\tTT\t$$\nread4\t4\t*\t0\t0\t*\t*\t0\t0\tAA\t%%\n",
        )
        .expect("sam fixture should write");

        let payload = run_impl(&ExplodeRequest {
            input: input.clone(),
            out_dir: out_dir.clone(),
            explode: 2,
            threads: 0,
            force: true,
        })
        .expect("explode should succeed");

        assert_eq!(payload.format, "SAM");
        assert_eq!(payload.outputs.len(), 2);
        let first = fs::read_to_string(&payload.outputs[0].path).expect("first shard should read");
        assert!(first.starts_with("@HD"));
        assert!(first.contains("read1"));

        fs::remove_file(&input).expect("fixture should remove");
        for output in &payload.outputs {
            fs::remove_file(&output.path).expect("shard should remove");
        }
        fs::remove_dir(&out_dir).expect("directory should remove");
    }

    #[test]
    fn fastq_gz_explode_reports_gzi_planning_and_uneven_ranges() {
        let input = std::env::temp_dir().join(format!(
            "bamana-explode-fastq-gz-uneven-{}.fastq.gz",
            std::process::id()
        ));
        let out_dir = std::env::temp_dir().join(format!(
            "bamana-explode-fastq-gz-uneven-out-{}",
            std::process::id()
        ));
        let file = File::create(&input).expect("fixture should create");
        let mut encoder = GzEncoder::new(file, Compression::default());
        for index in 0..5 {
            writeln!(encoder, "@read{index}").expect("header should write");
            writeln!(encoder, "ACGT").expect("sequence should write");
            writeln!(encoder, "+").expect("plus should write");
            writeln!(encoder, "!!!!").expect("quality should write");
        }
        encoder.finish().expect("gzip should finish");

        let payload = run_impl(&ExplodeRequest {
            input: input.clone(),
            out_dir: out_dir.clone(),
            explode: 2,
            threads: 2,
            force: true,
        })
        .expect("explode should succeed");

        assert_eq!(payload.format, "FASTQ.GZ");
        assert_eq!(payload.outputs.len(), 2);
        assert_eq!(payload.explode.total_records, Some(5));
        assert!(payload.explode.checkpoints_available.unwrap_or_default() > 0);
        let written = payload
            .outputs
            .iter()
            .map(|output| output.records_written)
            .collect::<Vec<_>>();
        assert_eq!(written.iter().sum::<u64>(), 5);
        assert_ne!(written[0], written[1]);
        assert!(payload.notes.iter().any(|note| note.contains("FASTQ.GZI")));

        fs::remove_file(&input).expect("fixture should remove");
        for output in &payload.outputs {
            fs::remove_file(&output.path).expect("shard should remove");
        }
        fs::remove_file(crate::fastq::gzi::fastq_gzi_output_path(&input))
            .expect("index should remove");
        fs::remove_dir(&out_dir).expect("directory should remove");
    }

    fn read_bam_names(path: &str) -> Vec<String> {
        let mut scanner =
            BamScanner::open(Path::new(path)).expect("BAM shard should open through scanner");
        let mut names = Vec::new();
        while let Some(record) = scanner.next_record().expect("record should parse") {
            names.push(record.read_name().to_string());
        }
        names
    }
}
