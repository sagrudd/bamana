use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
};

use crate::{
    bam::records::{
        BAM_FUNMAP, RecordLayout, encode_bam_qualities, encode_bam_sequence, missing_quality_scores,
    },
    error::AppError,
};

use super::{
    gzip::{is_gzip_fastq_path, resolved_threads},
    reader::{open_fastq_reader_with_label, read_next_fastq_record},
    record::{FastqRecord, parse_read_name},
};

pub fn read_fastq_as_unmapped_records(
    path: &Path,
    read_group: Option<&str>,
) -> Result<Vec<RecordLayout>, AppError> {
    read_fastq_as_unmapped_records_with_label(path, path, read_group)
}

pub fn read_fastq_as_unmapped_records_with_label(
    path: &Path,
    label: &Path,
    read_group: Option<&str>,
) -> Result<Vec<RecordLayout>, AppError> {
    read_fastq_as_unmapped_records_threaded_with_label(path, label, read_group, 1, None)
}

pub fn read_fastq_as_unmapped_records_threaded_with_label(
    path: &Path,
    label: &Path,
    read_group: Option<&str>,
    threads: usize,
    total_records_hint: Option<u64>,
) -> Result<Vec<RecordLayout>, AppError> {
    if is_gzip_fastq_path(path) && resolved_threads(threads) > 1 {
        return read_fastq_gz_as_unmapped_records_parallel_with_label(
            path,
            label,
            read_group,
            threads,
            total_records_hint,
        );
    }

    read_fastq_as_unmapped_records_serial_with_label(path, label, read_group)
}

fn read_fastq_as_unmapped_records_serial_with_label(
    path: &Path,
    label: &Path,
    read_group: Option<&str>,
) -> Result<Vec<RecordLayout>, AppError> {
    let mut reader = open_fastq_reader_with_label(path, label)?;
    let mut records = Vec::new();

    loop {
        let Some(record) = read_next_fastq_record(&mut reader, label)? else {
            break;
        };
        records.push(build_unmapped_record(
            label,
            &record.raw_header_line,
            &record.sequence,
            &record.plus_line,
            &record.quality,
            read_group,
        )?);
    }

    Ok(records)
}

fn read_fastq_gz_as_unmapped_records_parallel_with_label(
    path: &Path,
    label: &Path,
    read_group: Option<&str>,
    threads: usize,
    total_records_hint: Option<u64>,
) -> Result<Vec<RecordLayout>, AppError> {
    let resolved = resolved_threads(threads);
    if resolved <= 1 {
        return read_fastq_as_unmapped_records_serial_with_label(path, label, read_group);
    }

    let worker_count = resolved.saturating_sub(1).max(1);
    let batch_records = batch_record_target(total_records_hint, worker_count);
    let (batch_tx, batch_rx) = mpsc::sync_channel::<Vec<FastqRecord>>(worker_count * 2);
    let shared_rx = Arc::new(Mutex::new(batch_rx));
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let rx = Arc::clone(&shared_rx);
        let worker_label = label.to_path_buf();
        let worker_read_group = read_group.map(|value| value.to_string());
        handles.push(thread::spawn(
            move || -> Result<Vec<RecordLayout>, AppError> {
                let mut layouts = Vec::new();
                loop {
                    let batch = {
                        let lock = rx.lock().map_err(|_| AppError::Internal {
                            message: "FASTQ worker queue mutex was poisoned.".to_string(),
                        })?;
                        match lock.recv() {
                            Ok(batch) => batch,
                            Err(_) => break,
                        }
                    };

                    for record in batch {
                        layouts.push(build_unmapped_record(
                            &worker_label,
                            &record.raw_header_line,
                            &record.sequence,
                            &record.plus_line,
                            &record.quality,
                            worker_read_group.as_deref(),
                        )?);
                    }
                }
                Ok(layouts)
            },
        ));
    }

    let mut reader = open_fastq_reader_with_label(path, label)?;
    let mut batch = Vec::with_capacity(batch_records);
    loop {
        let Some(record) = read_next_fastq_record(&mut reader, label)? else {
            break;
        };
        batch.push(record);
        if batch.len() >= batch_records {
            if batch_tx
                .send(std::mem::replace(
                    &mut batch,
                    Vec::with_capacity(batch_records),
                ))
                .is_err()
            {
                return Err(AppError::Internal {
                    message: "FASTQ worker queue closed before parsing completed.".to_string(),
                });
            }
        }
    }

    if !batch.is_empty() && batch_tx.send(batch).is_err() {
        return Err(AppError::Internal {
            message: "FASTQ worker queue closed before parsing completed.".to_string(),
        });
    }
    drop(batch_tx);

    let mut records = Vec::new();
    for handle in handles {
        let mut worker_records = handle.join().map_err(|_| AppError::Internal {
            message: "FASTQ parser worker panicked.".to_string(),
        })??;
        records.append(&mut worker_records);
    }

    Ok(records)
}

fn build_unmapped_record(
    path: &Path,
    header_line: &str,
    sequence_line: &str,
    _plus_line: &str,
    quality_line: &str,
    read_group: Option<&str>,
) -> Result<RecordLayout, AppError> {
    let read_name = parse_read_name(header_line).ok_or_else(|| AppError::InvalidFastq {
        path: path.to_path_buf(),
        detail: "FASTQ record header did not contain a usable read name.".to_string(),
    })?;
    let sequence_bytes =
        encode_bam_sequence(sequence_line).map_err(|detail| AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail,
        })?;
    let quality_bytes =
        encode_bam_qualities(quality_line).map_err(|detail| AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail,
        })?;

    let mut aux_bytes = parse_methylation_fastq_header_aux(path, header_line, sequence_line.len())?;
    if let Some(read_group) = read_group {
        aux_bytes.extend_from_slice(&encode_read_group_aux(read_group));
    }
    let block_size =
        32 + read_name.len() + 1 + sequence_bytes.len() + quality_bytes.len() + aux_bytes.len();

    Ok(RecordLayout {
        block_size,
        ref_id: -1,
        pos: -1,
        bin: 4680,
        next_ref_id: -1,
        next_pos: -1,
        tlen: 0,
        flags: BAM_FUNMAP,
        mapping_quality: 0,
        n_cigar_op: 0,
        l_seq: sequence_line.len(),
        read_name,
        cigar_bytes: Vec::new(),
        sequence_bytes,
        quality_bytes: if quality_line == "*" {
            missing_quality_scores(sequence_line.len())
        } else {
            quality_bytes
        },
        aux_bytes,
    })
}

fn batch_record_target(total_records_hint: Option<u64>, worker_count: usize) -> usize {
    let Some(total_records) = total_records_hint else {
        return 4096;
    };
    let per_worker = total_records / worker_count.max(1) as u64;
    per_worker.clamp(1024, 16384) as usize
}

fn encode_read_group_aux(read_group: &str) -> Vec<u8> {
    let mut aux = Vec::with_capacity(5 + read_group.len());
    aux.extend_from_slice(b"RG");
    aux.push(b'Z');
    aux.extend_from_slice(read_group.as_bytes());
    aux.push(0);
    aux
}

fn parse_methylation_fastq_header_aux(
    path: &Path,
    header_line: &str,
    sequence_len: usize,
) -> Result<Vec<u8>, AppError> {
    let mut aux = Vec::new();
    let Some(rest) = header_line.strip_prefix('@') else {
        return Ok(aux);
    };
    let mut fields = rest.split_whitespace();
    let _read_name = fields.next();

    for field in fields {
        if !(field.starts_with("MM:") || field.starts_with("ML:") || field.starts_with("MN:")) {
            continue;
        }

        let parsed = parse_single_hts_header_tag(path, field, sequence_len)?;
        aux.extend_from_slice(&parsed);
    }

    Ok(aux)
}

fn parse_single_hts_header_tag(
    path: &Path,
    field: &str,
    sequence_len: usize,
) -> Result<Vec<u8>, AppError> {
    let mut parts = field.splitn(3, ':');
    let tag = parts.next().unwrap_or_default();
    let type_code = parts.next().ok_or_else(|| AppError::InvalidFastq {
        path: path.to_path_buf(),
        detail: format!("FASTQ header tag {field} was malformed."),
    })?;
    let value = parts.next().ok_or_else(|| AppError::InvalidFastq {
        path: path.to_path_buf(),
        detail: format!("FASTQ header tag {field} was malformed."),
    })?;

    match (tag, type_code) {
        ("MM", "Z") => {
            let mut bytes = Vec::with_capacity(3 + value.len() + 1);
            bytes.extend_from_slice(b"MM");
            bytes.push(b'Z');
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(0);
            Ok(bytes)
        }
        ("ML", "B") => parse_ml_header_tag(path, value),
        ("MN", "i") | ("MN", "I") => {
            let parsed = value.parse::<i32>().map_err(|_| AppError::InvalidFastq {
                path: path.to_path_buf(),
                detail: format!("FASTQ header MN tag did not contain a valid integer: {value}"),
            })?;
            if parsed < 0 {
                return Err(AppError::InvalidFastq {
                    path: path.to_path_buf(),
                    detail: "FASTQ header MN tag may not be negative.".to_string(),
                });
            }
            if parsed as usize != sequence_len {
                return Err(AppError::InvalidFastq {
                    path: path.to_path_buf(),
                    detail: format!(
                        "FASTQ header MN tag reported sequence length {parsed}, but the FASTQ sequence length was {sequence_len}."
                    ),
                });
            }
            let mut bytes = Vec::with_capacity(7);
            bytes.extend_from_slice(b"MN");
            bytes.push(b'i');
            bytes.extend_from_slice(&parsed.to_le_bytes());
            Ok(bytes)
        }
        ("MM", _) | ("ML", _) | ("MN", _) => Err(AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: format!(
                "FASTQ header methylation tag {tag} used unsupported type code {type_code}."
            ),
        }),
        _ => Ok(Vec::new()),
    }
}

fn parse_ml_header_tag(path: &Path, value: &str) -> Result<Vec<u8>, AppError> {
    let Some((subtype, values)) = value.split_once(',') else {
        return Err(AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: "FASTQ header ML tag must use B-array syntax such as ML:B:C,42,7.".to_string(),
        });
    };

    if subtype != "C" {
        return Err(AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: format!(
                "FASTQ header ML tag used unsupported B-array subtype {subtype}; only C is supported."
            ),
        });
    }

    let parsed_values = if values.is_empty() {
        Vec::new()
    } else {
        values
            .split(',')
            .map(|entry| {
                entry.parse::<u8>().map_err(|_| AppError::InvalidFastq {
                    path: path.to_path_buf(),
                    detail: format!(
                        "FASTQ header ML tag contained an invalid probability byte: {entry}"
                    ),
                })
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    let mut bytes = Vec::with_capacity(8 + parsed_values.len());
    bytes.extend_from_slice(b"ML");
    bytes.push(b'B');
    bytes.push(b'C');
    bytes.extend_from_slice(&(parsed_values.len() as i32).to_le_bytes());
    bytes.extend_from_slice(&parsed_values);
    Ok(bytes)
}
