use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
};

use flate2::{Compression, GzBuilder};

use crate::{
    bam::{
        reader::BamReader,
        records::{
            RecordLayout, decode_bam_qualities, decode_bam_sequence, read_next_record_layout,
        },
        tags::{AuxField, traverse_aux_fields},
    },
    error::AppError,
};

const TARGET_BATCH_BAM_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct FastqExportOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub threads: usize,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct FastqExportExecution {
    pub overwritten: bool,
    pub records_read: u64,
    pub records_written: u64,
    pub threads_used: usize,
    pub notes: Vec<String>,
}

#[derive(Debug)]
struct BatchJob {
    index: usize,
    records: Vec<RecordLayout>,
}

#[derive(Debug)]
struct BatchResult {
    index: usize,
    record_count: u64,
    compressed: Result<Vec<u8>, AppError>,
}

pub fn export_bam_to_fastq_gz(
    options: &FastqExportOptions,
) -> Result<FastqExportExecution, AppError> {
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

    let threads_used = resolved_threads(options.threads);
    let temp_path = temporary_output_path(&options.output_path);
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    let write_result = (|| -> Result<(u64, u64), AppError> {
        let mut reader = BamReader::open(&options.input_path)?;
        let _header = crate::bam::header::parse_bam_header_from_reader(&mut reader)?;
        let mut writer =
            BufWriter::new(
                File::create(&temp_path).map_err(|error| AppError::WriteError {
                    path: temp_path.clone(),
                    message: error.to_string(),
                })?,
            );

        let (job_tx, job_rx) = mpsc::channel::<BatchJob>();
        let (result_tx, result_rx) = mpsc::channel::<BatchResult>();
        let shared_rx = Arc::new(Mutex::new(job_rx));

        let mut handles = Vec::with_capacity(threads_used);
        for _ in 0..threads_used {
            let job_rx = Arc::clone(&shared_rx);
            let result_tx = result_tx.clone();
            let input_path = options.input_path.clone();
            handles.push(thread::spawn(move || {
                while let Some(job) = recv_job(&job_rx) {
                    let record_count = job.records.len() as u64;
                    let compressed = compress_batch(job.records, &input_path);
                    if result_tx
                        .send(BatchResult {
                            index: job.index,
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

        let mut batch_index = 0_usize;
        let mut batches_sent = 0_usize;
        let mut records_read = 0_u64;
        let mut records_written = 0_u64;
        let mut current_batch = Vec::new();
        let mut current_batch_bytes = 0_usize;
        let mut pending = BTreeMap::new();
        let mut next_to_write = 0_usize;

        while let Some(layout) = read_next_record_layout(&mut reader)? {
            current_batch_bytes += layout.block_size.max(
                32 + layout.read_name.len()
                    + layout.sequence_bytes.len()
                    + layout.quality_bytes.len(),
            );
            current_batch.push(layout);
            records_read += 1;

            if current_batch_bytes >= TARGET_BATCH_BAM_BYTES {
                submit_batch(
                    &job_tx,
                    batch_index,
                    &mut current_batch,
                    &mut current_batch_bytes,
                    &mut batches_sent,
                )?;
                batch_index += 1;
                drain_available_results(
                    &result_rx,
                    &mut pending,
                    &mut writer,
                    &mut next_to_write,
                    &mut records_written,
                    &temp_path,
                )?;
            }
        }

        if !current_batch.is_empty() {
            submit_batch(
                &job_tx,
                batch_index,
                &mut current_batch,
                &mut current_batch_bytes,
                &mut batches_sent,
            )?;
        }
        drop(job_tx);

        while next_to_write < batches_sent {
            let result = result_rx.recv().map_err(|_| AppError::Internal {
                message: "FASTQ export worker channel closed before all batches completed."
                    .to_string(),
            })?;
            pending.insert(result.index, (result.record_count, result.compressed));
            write_ready_batches(
                &mut pending,
                &mut writer,
                &mut next_to_write,
                &mut records_written,
                &temp_path,
            )?;
        }

        writer.flush().map_err(|error| AppError::WriteError {
            path: temp_path.clone(),
            message: error.to_string(),
        })?;

        for handle in handles {
            handle.join().map_err(|_| AppError::Internal {
                message: "FASTQ export worker thread panicked.".to_string(),
            })?;
        }

        Ok((records_read, records_written))
    })();

    let (records_read, records_written) = match write_result {
        Ok(counts) => counts,
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
        "FASTQ.GZ output is written as an ordered stream of concatenated gzip members so decode and compression work can spread across multiple CPU cores."
            .to_string(),
        "Read names, sequences, and qualities are emitted in input BAM order; BAM header content is not represented in FASTQ output."
            .to_string(),
    ];
    if threads_used > 1 {
        notes.push(format!(
            "FASTQ export used {} worker threads for parallel decode/compression.",
            threads_used
        ));
    } else {
        notes.push("FASTQ export used a single worker thread because only one CPU core was available or requested.".to_string());
    }

    Ok(FastqExportExecution {
        overwritten: preexisting_output && options.force,
        records_read,
        records_written,
        threads_used,
        notes,
    })
}

fn recv_job(job_rx: &Arc<Mutex<mpsc::Receiver<BatchJob>>>) -> Option<BatchJob> {
    job_rx.lock().ok()?.recv().ok()
}

fn submit_batch(
    job_tx: &mpsc::Sender<BatchJob>,
    batch_index: usize,
    current_batch: &mut Vec<RecordLayout>,
    current_batch_bytes: &mut usize,
    batches_sent: &mut usize,
) -> Result<(), AppError> {
    let records = std::mem::take(current_batch);
    *current_batch_bytes = 0;
    job_tx
        .send(BatchJob {
            index: batch_index,
            records,
        })
        .map_err(|_| AppError::Internal {
            message: "FASTQ export worker channel closed unexpectedly.".to_string(),
        })?;
    *batches_sent += 1;
    Ok(())
}

fn drain_available_results(
    result_rx: &mpsc::Receiver<BatchResult>,
    pending: &mut BTreeMap<usize, (u64, Result<Vec<u8>, AppError>)>,
    writer: &mut BufWriter<File>,
    next_to_write: &mut usize,
    records_written: &mut u64,
    path: &Path,
) -> Result<(), AppError> {
    while let Ok(result) = result_rx.try_recv() {
        pending.insert(result.index, (result.record_count, result.compressed));
    }
    write_ready_batches(pending, writer, next_to_write, records_written, path)
}

fn write_ready_batches(
    pending: &mut BTreeMap<usize, (u64, Result<Vec<u8>, AppError>)>,
    writer: &mut BufWriter<File>,
    next_to_write: &mut usize,
    records_written: &mut u64,
    path: &Path,
) -> Result<(), AppError> {
    while let Some((record_count, result)) = pending.remove(next_to_write) {
        let compressed = result?;
        writer
            .write_all(&compressed)
            .map_err(|error| AppError::WriteError {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        *records_written += record_count;
        *next_to_write += 1;
    }
    Ok(())
}

fn compress_batch(records: Vec<RecordLayout>, input_path: &Path) -> Result<Vec<u8>, AppError> {
    let mut payload = Vec::new();
    for record in records {
        append_fastq_record(&mut payload, &record, input_path)?;
    }

    let mut encoder = GzBuilder::new().write(Vec::new(), Compression::fast());
    encoder
        .write_all(&payload)
        .map_err(|error| AppError::WriteError {
            path: input_path.to_path_buf(),
            message: error.to_string(),
        })?;
    encoder.finish().map_err(|error| AppError::WriteError {
        path: input_path.to_path_buf(),
        message: error.to_string(),
    })
}

fn append_fastq_record(
    output: &mut Vec<u8>,
    record: &RecordLayout,
    input_path: &Path,
) -> Result<(), AppError> {
    let sequence = decode_bam_sequence(&record.sequence_bytes, record.l_seq).map_err(|detail| {
        AppError::ParseUncertainty {
            path: input_path.to_path_buf(),
            detail,
        }
    })?;
    let quality = decode_bam_qualities(&record.quality_bytes).map_err(|detail| {
        AppError::ParseUncertainty {
            path: input_path.to_path_buf(),
            detail,
        }
    })?;

    output.push(b'@');
    output.extend_from_slice(record.read_name.as_bytes());
    append_methylation_header_tags(output, &record.aux_bytes, input_path)?;
    output.push(b'\n');
    output.extend_from_slice(sequence.as_bytes());
    output.extend_from_slice(b"\n+\n");
    if quality == "*" {
        output.extend(std::iter::repeat_n(b'!', record.l_seq));
    } else {
        output.extend_from_slice(quality.as_bytes());
    }
    output.push(b'\n');
    Ok(())
}

fn append_methylation_header_tags(
    output: &mut Vec<u8>,
    aux_bytes: &[u8],
    input_path: &Path,
) -> Result<(), AppError> {
    let mut tags = Vec::new();
    traverse_aux_fields(aux_bytes, |field| {
        if matches!(field.tag, [b'M', b'M'] | [b'M', b'L'] | [b'M', b'N']) {
            tags.push(format_aux_field(field)?);
        }
        Ok(())
    })
    .map_err(|detail| AppError::TagParseUncertainty {
        path: input_path.to_path_buf(),
        detail,
    })?;

    for tag in tags {
        output.push(b' ');
        output.extend_from_slice(tag.as_bytes());
    }

    Ok(())
}

fn format_aux_field(field: AuxField<'_>) -> Result<String, String> {
    let tag = String::from_utf8(vec![field.tag[0], field.tag[1]])
        .map_err(|error| format!("BAM auxiliary tag name was not valid UTF-8: {error}"))?;
    let value = match field.type_code {
        b'A' => {
            let value = *field
                .payload
                .first()
                .ok_or_else(|| "Auxiliary A tag payload was empty.".to_string())?;
            format!("A:{}", char::from(value))
        }
        b'c' => format!(
            "c:{}",
            i8::from_le_bytes([require_payload(field.payload, 1)?[0]])
        ),
        b'C' => format!("C:{}", require_payload(field.payload, 1)?[0]),
        b's' => {
            let bytes = require_payload(field.payload, 2)?;
            format!("s:{}", i16::from_le_bytes([bytes[0], bytes[1]]))
        }
        b'S' => {
            let bytes = require_payload(field.payload, 2)?;
            format!("S:{}", u16::from_le_bytes([bytes[0], bytes[1]]))
        }
        b'i' => {
            let bytes = require_payload(field.payload, 4)?;
            format!(
                "i:{}",
                i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            )
        }
        b'I' => {
            let bytes = require_payload(field.payload, 4)?;
            format!(
                "I:{}",
                u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            )
        }
        b'f' => {
            let bytes = require_payload(field.payload, 4)?;
            format!(
                "f:{}",
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            )
        }
        b'Z' | b'H' => {
            let value = field.payload.strip_suffix(&[0]).ok_or_else(|| {
                "Encountered a malformed NUL-terminated auxiliary string.".to_string()
            })?;
            let value = String::from_utf8(value.to_vec()).map_err(|error| {
                format!("BAM auxiliary string tag was not valid UTF-8: {error}")
            })?;
            format!("{}:{}", field.type_code as char, value)
        }
        b'B' => format_b_array(field.payload)?,
        other => {
            return Err(format!(
                "Encountered unsupported or malformed BAM auxiliary type code '{}'.",
                other as char
            ));
        }
    };

    Ok(format!("{tag}:{value}"))
}

fn format_b_array(payload: &[u8]) -> Result<String, String> {
    if payload.len() < 5 {
        return Err(
            "Encountered a truncated auxiliary field before a stable conclusion was reached."
                .to_string(),
        );
    }

    let subtype = payload[0] as char;
    let count = i32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
    if count < 0 {
        return Err(
            "Encountered a BAM auxiliary B-array with a negative element count.".to_string(),
        );
    }
    let count = count as usize;
    let values = &payload[5..];

    let rendered = match payload[0] {
        b'c' => values
            .iter()
            .take(count)
            .map(|value| i8::from_le_bytes([*value]).to_string())
            .collect::<Vec<_>>(),
        b'C' => values
            .iter()
            .take(count)
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        b's' => values
            .chunks_exact(2)
            .take(count)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]).to_string())
            .collect::<Vec<_>>(),
        b'S' => values
            .chunks_exact(2)
            .take(count)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]).to_string())
            .collect::<Vec<_>>(),
        b'i' => values
            .chunks_exact(4)
            .take(count)
            .map(|chunk| i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_string())
            .collect::<Vec<_>>(),
        b'I' => values
            .chunks_exact(4)
            .take(count)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_string())
            .collect::<Vec<_>>(),
        b'f' => values
            .chunks_exact(4)
            .take(count)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_string())
            .collect::<Vec<_>>(),
        other => {
            return Err(format!(
                "Encountered unsupported BAM auxiliary B-array subtype '{}'.",
                other as char
            ));
        }
    };

    Ok(format!("B:{subtype},{}", rendered.join(",")))
}

fn require_payload(payload: &[u8], expected: usize) -> Result<&[u8], String> {
    if payload.len() < expected {
        return Err(
            "Encountered a truncated auxiliary field before a stable conclusion was reached."
                .to_string(),
        );
    }
    Ok(&payload[..expected])
}

fn resolved_threads(requested_threads: usize) -> usize {
    let available = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    if requested_threads == 0 {
        available.max(1)
    } else {
        requested_threads.min(available).max(1)
    }
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
        .unwrap_or("bamana-fastq-output");
    output.with_file_name(format!(".{stem}.bamana-fastq-{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        bam::{
            fastq::{FastqExportOptions, append_fastq_record, export_bam_to_fastq_gz},
            records::{
                RecordLayout, encode_bam_qualities, encode_bam_sequence, missing_quality_scores,
            },
        },
        fastq::{open_fastq_reader, read_next_fastq_record},
        formats::bgzf::test_support::{build_bam_file_with_header_and_records, write_temp_file},
    };

    #[test]
    fn export_writes_gzipped_fastq_in_input_order() {
        let first = RecordLayout {
            block_size: 0,
            ref_id: 0,
            pos: 1,
            bin: 0,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags: 0,
            mapping_quality: 60,
            n_cigar_op: 0,
            l_seq: 4,
            read_name: "read1".to_string(),
            cigar_bytes: Vec::new(),
            sequence_bytes: encode_bam_sequence("ACGT").expect("seq should encode"),
            quality_bytes: encode_bam_qualities("!!!!").expect("qual should encode"),
            aux_bytes: Vec::new(),
        };
        let second = RecordLayout {
            block_size: 0,
            ref_id: 0,
            pos: 2,
            bin: 0,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags: 0,
            mapping_quality: 60,
            n_cigar_op: 0,
            l_seq: 2,
            read_name: "read2".to_string(),
            cigar_bytes: Vec::new(),
            sequence_bytes: encode_bam_sequence("TG").expect("seq should encode"),
            quality_bytes: encode_bam_qualities("##").expect("qual should encode"),
            aux_bytes: Vec::new(),
        };
        let input = write_temp_file(
            "fastq-export-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    crate::bam::write::serialize_record_layout(&first),
                    crate::bam::write::serialize_record_layout(&second),
                ],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-fastq-export-{}.fastq.gz",
            std::process::id()
        ));

        let execution = export_bam_to_fastq_gz(&FastqExportOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            threads: 2,
            force: true,
        })
        .expect("export should succeed");

        assert_eq!(execution.records_read, 2);
        let mut reader = open_fastq_reader(&output).expect("fastq.gz should open");
        let first_record = read_next_fastq_record(&mut reader, &output)
            .expect("first record should parse")
            .expect("first record should exist");
        let second_record = read_next_fastq_record(&mut reader, &output)
            .expect("second record should parse")
            .expect("second record should exist");
        assert_eq!(first_record.read_name, "read1");
        assert_eq!(first_record.sequence, "ACGT");
        assert_eq!(second_record.read_name, "read2");

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn missing_bam_qualities_become_fastq_bang_scores() {
        let record = RecordLayout {
            block_size: 0,
            ref_id: -1,
            pos: -1,
            bin: 4680,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags: 4,
            mapping_quality: 0,
            n_cigar_op: 0,
            l_seq: 3,
            read_name: "readq".to_string(),
            cigar_bytes: Vec::new(),
            sequence_bytes: encode_bam_sequence("AAA").expect("seq should encode"),
            quality_bytes: missing_quality_scores(3),
            aux_bytes: Vec::new(),
        };

        let mut bytes = Vec::new();
        append_fastq_record(&mut bytes, &record, Path::new("input.bam"))
            .expect("fastq append should succeed");

        let text = String::from_utf8(bytes).expect("fastq bytes should be utf8");
        assert!(text.ends_with("!!!\n"));
    }

    #[test]
    fn methylation_tags_are_preserved_in_fastq_header() {
        let record = RecordLayout {
            block_size: 0,
            ref_id: -1,
            pos: -1,
            bin: 4680,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags: 4,
            mapping_quality: 0,
            n_cigar_op: 0,
            l_seq: 2,
            read_name: "modread".to_string(),
            cigar_bytes: Vec::new(),
            sequence_bytes: encode_bam_sequence("AC").expect("seq should encode"),
            quality_bytes: encode_bam_qualities("!!").expect("qual should encode"),
            aux_bytes: vec![
                b'M', b'M', b'Z', b'C', b'+', b'm', b',', b'0', b';', 0, b'M', b'L', b'B', b'C', 2,
                0, 0, 0, 42, 7, b'M', b'N', b'i', 2, 0, 0, 0,
            ],
        };

        let mut bytes = Vec::new();
        append_fastq_record(&mut bytes, &record, Path::new("input.bam"))
            .expect("fastq append should succeed");
        let text = String::from_utf8(bytes).expect("fastq bytes should be utf8");

        assert!(text.starts_with("@modread MM:Z:C+m,0; ML:B:C,42,7 MN:i:2\n"));
    }
}
