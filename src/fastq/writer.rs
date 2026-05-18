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
        for line in [
            &record.raw_header_line,
            &record.sequence,
            &record.plus_line,
            &record.quality,
        ] {
            self.write_all(line.as_bytes())?;
            self.write_all(b"\n")?;
        }

        Ok(())
    }

    pub fn finish(mut self) -> Result<(), AppError> {
        match &mut self.inner {
            FastqWriterInner::Plain(writer) => {
                writer.flush().map_err(|error| AppError::WriteError {
                    path: self.path.clone(),
                    message: error.to_string(),
                })
            }
            FastqWriterInner::Gzip(encoder) => {
                encoder.try_finish().map_err(|error| AppError::WriteError {
                    path: self.path.clone(),
                    message: error.to_string(),
                })?;
                Ok(())
            }
        }
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), AppError> {
        match &mut self.inner {
            FastqWriterInner::Plain(writer) => writer.write_all(bytes),
            FastqWriterInner::Gzip(encoder) => encoder.write_all(bytes),
        }
        .map_err(|error| AppError::WriteError {
            path: self.path.clone(),
            message: error.to_string(),
        })
    }
}
