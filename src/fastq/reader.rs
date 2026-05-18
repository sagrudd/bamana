use std::{
    io::BufRead,
    path::{Path, PathBuf},
};

use crate::error::AppError;

use super::{gzip::open_maybe_gzip_reader, record::FastqRecord};

pub fn count_fastq_records(path: &Path) -> Result<u64, AppError> {
    count_fastq_records_with_label(path, path)
}

pub fn count_fastq_records_with_label(path: &Path, label: &Path) -> Result<u64, AppError> {
    let mut reader = open_fastq_reader_with_label(path, label)?;
    let mut records = 0_u64;

    loop {
        let Some(_record) = read_next_fastq_record(&mut reader, label)? else {
            break;
        };
        records += 1;
    }

    Ok(records)
}

pub fn open_fastq_reader(path: &Path) -> Result<Box<dyn BufRead>, AppError> {
    open_fastq_reader_with_label(path, path)
}

pub fn open_fastq_reader_with_label(
    path: &Path,
    label: &Path,
) -> Result<Box<dyn BufRead>, AppError> {
    open_maybe_gzip_reader(path, label)
}

pub fn read_next_fastq_record(
    reader: &mut dyn BufRead,
    path: &Path,
) -> Result<Option<FastqRecord>, AppError> {
    let Some(header_line) = read_next_line(reader, path)? else {
        return Ok(None);
    };
    let sequence_line = required_line(reader, path, "sequence")?;
    let plus_line = required_line(reader, path, "plus")?;
    let quality_line = required_line(reader, path, "quality")?;

    FastqRecord::from_lines(header_line, sequence_line, plus_line, quality_line)
        .map(Some)
        .map_err(|error| AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: error.detail().to_string(),
        })
}

fn read_next_line(reader: &mut dyn BufRead, path: &Path) -> Result<Option<String>, AppError> {
    let mut line = String::new();
    let bytes_read = reader
        .read_line(&mut line)
        .map_err(|error| AppError::from_io(path, error))?;
    if bytes_read == 0 {
        return Ok(None);
    }
    Ok(Some(trim_line_endings(line)))
}

fn required_line(reader: &mut dyn BufRead, path: &Path, label: &str) -> Result<String, AppError> {
    read_next_line(reader, path)?.ok_or_else(|| AppError::InvalidFastq {
        path: PathBuf::from(path),
        detail: format!("FASTQ ended before the {label} line of a complete record was available."),
    })
}

fn trim_line_endings(mut line: String) -> String {
    while line.ends_with(['\n', '\r']) {
        line.pop();
    }
    line
}
