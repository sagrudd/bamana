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

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::Path};

    use crate::error::AppError;

    use super::read_next_fastq_record;

    fn read_all(input: &[u8]) -> Result<Vec<crate::fastq::FastqRecord>, AppError> {
        let path = Path::new("reader.fastq");
        let mut reader = Cursor::new(input);
        let mut records = Vec::new();
        loop {
            let Some(record) = read_next_fastq_record(&mut reader, path)? else {
                break;
            };
            records.push(record);
        }
        Ok(records)
    }

    fn expect_invalid(input: &[u8], expected_detail: &str) {
        let path = Path::new("reader.fastq");
        let mut reader = Cursor::new(input);
        let error = read_next_fastq_record(&mut reader, path)
            .expect_err("reader should reject malformed FASTQ");
        match error {
            AppError::InvalidFastq { path, detail } => {
                assert_eq!(path, Path::new("reader.fastq"));
                assert_eq!(detail, expected_detail);
            }
            other => panic!("expected InvalidFastq, got {other:?}"),
        }
    }

    #[test]
    fn reads_valid_single_record_and_then_eof() {
        let path = Path::new("single.fastq");
        let mut reader = Cursor::new(b"@read1 comment\nACGT\n+plus comment\n!!!!\n");

        let first = read_next_fastq_record(&mut reader, path)
            .expect("record should parse")
            .expect("record should exist");
        let eof = read_next_fastq_record(&mut reader, path).expect("EOF should be clean");

        assert_eq!(first.read_name, "read1");
        assert_eq!(first.raw_header_line, "@read1 comment");
        assert_eq!(first.sequence, "ACGT");
        assert_eq!(first.plus_line, "+plus comment");
        assert_eq!(first.quality, "!!!!");
        assert!(eof.is_none());
    }

    #[test]
    fn reads_valid_multi_record_plain_fastq_in_encounter_order() {
        let records = read_all(b"@read1\nAC\n+\n!!\n@read2 comment\nTGCA\n+two\n####\n")
            .expect("records should parse");

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].read_name, "read1");
        assert_eq!(records[0].sequence, "AC");
        assert_eq!(records[1].read_name, "read2");
        assert_eq!(records[1].raw_header_line, "@read2 comment");
        assert_eq!(records[1].plus_line, "+two");
        assert_eq!(records[1].quality, "####");
    }

    #[test]
    fn trims_crlf_line_endings_while_preserving_raw_fastq_semantics() {
        let records = read_all(b"@read1 comment\r\nACGT\r\n+plus comment\r\n!!!!\r\n")
            .expect("CRLF record should parse");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].raw_header_line, "@read1 comment");
        assert_eq!(records[0].sequence, "ACGT");
        assert_eq!(records[0].plus_line, "+plus comment");
        assert_eq!(records[0].quality, "!!!!");
    }

    #[test]
    fn empty_input_is_clean_eof() {
        let path = Path::new("empty.fastq");
        let mut reader = Cursor::new(Vec::<u8>::new());

        let eof = read_next_fastq_record(&mut reader, path).expect("EOF should be clean");

        assert!(eof.is_none());
    }

    #[test]
    fn rejects_truncated_records_at_each_missing_line_boundary() {
        expect_invalid(
            b"@read1\n",
            "FASTQ ended before the sequence line of a complete record was available.",
        );
        expect_invalid(
            b"@read1\nACGT\n",
            "FASTQ ended before the plus line of a complete record was available.",
        );
        expect_invalid(
            b"@read1\nACGT\n+\n",
            "FASTQ ended before the quality line of a complete record was available.",
        );
    }

    #[test]
    fn rejects_invalid_header_and_plus_markers() {
        expect_invalid(
            b"read1\nACGT\n+\n!!!!\n",
            "FASTQ record header line did not start with '@'.",
        );
        expect_invalid(
            b"@read1\nACGT\nplus\n!!!!\n",
            "FASTQ record plus line did not start with '+'.",
        );
    }

    #[test]
    fn rejects_sequence_quality_length_mismatch_and_blank_read_name() {
        expect_invalid(
            b"@read1\nACGT\n+\n!!!\n",
            "FASTQ sequence and quality lengths differed (4 vs 3).",
        );
        expect_invalid(
            b"@\nACGT\n+\n!!!!\n",
            "FASTQ record header did not contain a usable read name.",
        );
    }
}
