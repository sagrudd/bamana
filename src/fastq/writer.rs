use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use flate2::{Compression, write::GzEncoder};

use crate::error::AppError;

use super::{gzip::is_gzip_fastq_path, record::FastqRecord};

pub fn write_fastq_records(path: &Path, records: &[FastqRecord]) -> Result<(), AppError> {
    let mut writer = FastqWriter::create(path)?;
    for record in records {
        writer.write_record(record)?;
    }
    writer.finish()?;
    Ok(())
}

pub fn write_fastq_record_to<W: Write>(
    writer: &mut W,
    record: &FastqRecord,
    path: &Path,
) -> Result<(), AppError> {
    for line in record.view().lines() {
        writer
            .write_all(line.as_bytes())
            .and_then(|()| writer.write_all(b"\n"))
            .map_err(|error| AppError::WriteError {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
    }

    Ok(())
}

pub struct FastqWriter {
    path: PathBuf,
    inner: FastqWriterInner,
}

enum FastqWriterInner {
    Plain(BufWriter<File>),
    Gzip(GzEncoder<BufWriter<File>>),
}

impl FastqWriter {
    pub fn create(path: &Path) -> Result<Self, AppError> {
        let file = File::create(path).map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        let inner = if is_gzip_fastq_path(path) {
            FastqWriterInner::Gzip(GzEncoder::new(BufWriter::new(file), Compression::default()))
        } else {
            FastqWriterInner::Plain(BufWriter::new(file))
        };

        Ok(Self {
            path: path.to_path_buf(),
            inner,
        })
    }

    pub fn write_record(&mut self, record: &FastqRecord) -> Result<(), AppError> {
        match &mut self.inner {
            FastqWriterInner::Plain(writer) => write_fastq_record_to(writer, record, &self.path),
            FastqWriterInner::Gzip(encoder) => write_fastq_record_to(encoder, record, &self.path),
        }
    }

    pub fn finish(self) -> Result<(), AppError> {
        match self.inner {
            FastqWriterInner::Plain(mut writer) => {
                writer.flush().map_err(|error| AppError::WriteError {
                    path: self.path.clone(),
                    message: error.to_string(),
                })
            }
            FastqWriterInner::Gzip(mut encoder) => {
                encoder.try_finish().map_err(|error| AppError::WriteError {
                    path: self.path.clone(),
                    message: error.to_string(),
                })?;
                encoder
                    .get_mut()
                    .flush()
                    .map_err(|error| AppError::WriteError {
                        path: self.path.clone(),
                        message: error.to_string(),
                    })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use crate::{
        error::AppError,
        fastq::{FastqRecord, count_fastq_records, open_fastq_reader, read_next_fastq_record},
    };

    use super::{FastqWriter, write_fastq_records};

    fn temp_path(stem: &str, extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!("bamana-{stem}-{}-{extension}", std::process::id()))
    }

    fn remove_if_exists(path: &Path) {
        if path.exists() {
            fs::remove_file(path).expect("fixture should be removable");
        }
    }

    fn representative_records() -> Vec<FastqRecord> {
        vec![
            FastqRecord::from_lines(
                "@read1 run=42".to_string(),
                "ACGTN".to_string(),
                "+preserved plus comment".to_string(),
                "!!!!!".to_string(),
            )
            .expect("first record should validate"),
            FastqRecord::from_lines(
                "@read2".to_string(),
                "TGCA".to_string(),
                "+".to_string(),
                "####".to_string(),
            )
            .expect("second record should validate"),
        ]
    }

    fn read_records(path: &Path) -> Vec<FastqRecord> {
        let mut reader = open_fastq_reader(path).expect("reader should open writer output");
        let mut records = Vec::new();
        while let Some(record) =
            read_next_fastq_record(&mut reader, path).expect("writer output should parse")
        {
            records.push(record);
        }
        records
    }

    #[test]
    fn plain_writer_round_trips_records_and_uses_lf_line_endings() {
        let path = temp_path("plain-writer-round-trip", "fastq");
        let records = representative_records();

        write_fastq_records(&path, &records).expect("plain FASTQ should write");
        let bytes = fs::read(&path).expect("plain output should be readable");
        let parsed = read_records(&path);
        remove_if_exists(&path);

        assert_eq!(parsed, records);
        assert_eq!(
            bytes,
            b"@read1 run=42\nACGTN\n+preserved plus comment\n!!!!!\n@read2\nTGCA\n+\n####\n"
        );
    }

    #[test]
    fn gzip_writer_round_trips_records_and_finalizes_before_success() {
        let path = temp_path("gzip-writer-round-trip", "fastq.gz");
        let records = representative_records();

        write_fastq_records(&path, &records).expect("gzip FASTQ should write");
        let bytes = fs::read(&path).expect("gzip output should be readable");
        let count = count_fastq_records(&path).expect("finished gzip output should count");
        let parsed = read_records(&path);
        remove_if_exists(&path);

        assert_eq!(bytes.get(0..2), Some(&[0x1f, 0x8b][..]));
        assert_eq!(count, 2);
        assert_eq!(parsed, records);
    }

    #[test]
    fn output_compression_is_selected_by_final_gz_extension() {
        let plain_path = temp_path("writer-extension-plain", "gz.tmp");
        let gzip_path = temp_path("writer-extension-gzip", "FASTQ.GZ");
        let records = representative_records();

        write_fastq_records(&plain_path, &records).expect("plain extension output should write");
        write_fastq_records(&gzip_path, &records).expect("gzip extension output should write");
        let plain_bytes = fs::read(&plain_path).expect("plain output should be readable");
        let gzip_bytes = fs::read(&gzip_path).expect("gzip output should be readable");
        remove_if_exists(&plain_path);
        remove_if_exists(&gzip_path);

        assert_eq!(plain_bytes.get(0..2), Some(&b"@r"[..]));
        assert_ne!(plain_bytes.get(0..2), Some(&[0x1f, 0x8b][..]));
        assert_eq!(gzip_bytes.get(0..2), Some(&[0x1f, 0x8b][..]));
    }

    #[test]
    fn create_failure_reports_structured_write_error_with_output_path() {
        let path = std::env::temp_dir()
            .join(format!("bamana-missing-parent-{}", std::process::id()))
            .join("out.fastq");

        let error = match FastqWriter::create(&path) {
            Ok(_) => panic!("missing parent should fail"),
            Err(error) => error,
        };

        match error {
            AppError::WriteError {
                path: error_path,
                message,
            } => {
                assert_eq!(error_path, path);
                assert!(!message.is_empty());
            }
            other => panic!("expected structured write error, got {other:?}"),
        }
    }
}
