use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use flate2::read::MultiGzDecoder;
use flate2::{Compression, write::GzEncoder};

use crate::{
    bam::records::{
        BAM_FUNMAP, RecordLayout, encode_bam_qualities, encode_bam_sequence, missing_quality_scores,
    },
    error::AppError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastqRecord {
    pub raw_header_line: String,
    pub read_name: String,
    pub sequence: String,
    pub plus_line: String,
    pub quality: String,
}

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

pub fn count_fastq_records(path: &Path) -> Result<u64, AppError> {
    count_fastq_records_with_label(path, path)
}

pub fn count_fastq_records_with_label(path: &Path, label: &Path) -> Result<u64, AppError> {
    let mut reader = open_fastq_reader_with_label(path, label)?;
    let mut records = 0u64;

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
    let file = File::open(path).map_err(|error| AppError::from_io(label, error))?;
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
    {
        Ok(Box::new(BufReader::new(MultiGzDecoder::new(file))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
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

    if !header_line.starts_with('@') {
        return Err(AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: "FASTQ record header line did not start with '@'.".to_string(),
        });
    }
    if !plus_line.starts_with('+') {
        return Err(AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: "FASTQ record plus line did not start with '+'.".to_string(),
        });
    }
    if sequence_line.len() != quality_line.len() {
        return Err(AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: format!(
                "FASTQ sequence and quality lengths differed ({} vs {}).",
                sequence_line.len(),
                quality_line.len()
            ),
        });
    }

    let read_name = parse_read_name(&header_line).ok_or_else(|| AppError::InvalidFastq {
        path: path.to_path_buf(),
        detail: "FASTQ record header did not contain a usable read name.".to_string(),
    })?;

    Ok(Some(FastqRecord {
        raw_header_line: header_line,
        read_name,
        sequence: sequence_line,
        plus_line,
        quality: quality_line,
    }))
}

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

        let inner = if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
        {
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
        path: path.to_path_buf(),
        detail: format!("FASTQ ended before the {label} line of a complete record was available."),
    })
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

fn parse_read_name(header_line: &str) -> Option<String> {
    header_line
        .strip_prefix('@')?
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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

fn trim_line_endings(mut line: String) -> String {
    while line.ends_with(['\n', '\r']) {
        line.pop();
    }
    line
}

#[cfg(test)]
mod tests {
    use std::{fs, fs::File, io::Write};

    use flate2::{Compression, write::GzEncoder};

    use super::{
        FastqRecord, count_fastq_records, open_fastq_reader, read_fastq_as_unmapped_records,
        read_next_fastq_record, write_fastq_records,
    };

    #[test]
    fn parses_plain_fastq_into_unmapped_records() {
        let path =
            std::env::temp_dir().join(format!("bamana-fastq-plain-{}.fastq", std::process::id()));
        fs::write(&path, "@read1 comment\nACGT\n+\n!!!!\n").expect("fastq should write");

        let records =
            read_fastq_as_unmapped_records(&path, Some("rg1")).expect("fastq should parse");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].read_name, "read1");
        assert_eq!(records[0].l_seq, 4);
        assert_eq!(records[0].flags & 0x4, 0x4);
        assert!(records[0].aux_bytes.starts_with(b"RGZ"));
    }

    #[test]
    fn parses_gzipped_fastq_into_unmapped_records() {
        let path =
            std::env::temp_dir().join(format!("bamana-fastq-gzip-{}.fastq.gz", std::process::id()));
        let file = File::create(&path).expect("gzip fixture should open");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(b"@read2\nNN\n+\n##\n")
            .expect("gzip fixture should write");
        encoder.finish().expect("gzip fixture should finish");

        let records =
            read_fastq_as_unmapped_records(&path, None).expect("gzipped fastq should parse");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].read_name, "read2");
        assert_eq!(records[0].l_seq, 2);
    }

    #[test]
    fn counts_plain_and_gzipped_fastq_records_without_unpacking() {
        let plain_path =
            std::env::temp_dir().join(format!("bamana-fastq-count-{}.fastq", std::process::id()));
        fs::write(&plain_path, "@read1\nAC\n+\n!!\n@read2\nTG\n+\n##\n")
            .expect("plain fastq should write");

        let gzip_path = std::env::temp_dir().join(format!(
            "bamana-fastq-count-{}.fastq.gz",
            std::process::id()
        ));
        let file = File::create(&gzip_path).expect("gzip fixture should open");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(b"@read1\nAC\n+\n!!\n@read2\nTG\n+\n##\n")
            .expect("gzip fixture should write");
        encoder.finish().expect("gzip fixture should finish");

        let plain_count = count_fastq_records(&plain_path).expect("plain count should succeed");
        let gzip_count = count_fastq_records(&gzip_path).expect("gzip count should succeed");

        fs::remove_file(plain_path).expect("plain fixture should be removable");
        fs::remove_file(gzip_path).expect("gzip fixture should be removable");

        assert_eq!(plain_count, 2);
        assert_eq!(gzip_count, 2);
    }

    #[test]
    fn reads_structured_fastq_records() {
        let path =
            std::env::temp_dir().join(format!("bamana-fastq-record-{}.fastq", std::process::id()));
        fs::write(&path, "@read3 comment\nACGT\n+\n!!!!\n").expect("fastq should write");

        let mut reader = open_fastq_reader(&path).expect("reader should open");
        let record = read_next_fastq_record(&mut reader, &path)
            .expect("fastq record should parse")
            .expect("record should exist");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(record.read_name, "read3");
        assert_eq!(record.sequence, "ACGT");
        assert_eq!(record.raw_header_line, "@read3 comment");
        assert_eq!(record.plus_line, "+");
        assert_eq!(record.quality, "!!!!");
    }

    #[test]
    fn writes_fastq_records_with_original_structure() {
        let path =
            std::env::temp_dir().join(format!("bamana-fastq-write-{}.fastq", std::process::id()));
        let records = vec![FastqRecord {
            raw_header_line: "@read4 comment".to_string(),
            read_name: "read4".to_string(),
            sequence: "ACGT".to_string(),
            plus_line: "+comment".to_string(),
            quality: "!!!!".to_string(),
        }];

        write_fastq_records(&path, &records).expect("fastq records should write");
        let contents = fs::read_to_string(&path).expect("written fastq should be readable");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(contents, "@read4 comment\nACGT\n+comment\n!!!!\n");
    }

    #[test]
    fn parses_methylation_tags_from_hts_style_fastq_header() {
        let path =
            std::env::temp_dir().join(format!("bamana-fastq-methyl-{}.fastq", std::process::id()));
        fs::write(
            &path,
            "@modread MM:Z:C+m,0; ML:B:C,42,7 MN:i:4\nACGT\n+\n!!!!\n",
        )
        .expect("fastq should write");

        let records =
            read_fastq_as_unmapped_records(&path, None).expect("fastq should parse with mods");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(records.len(), 1);
        assert!(
            records[0]
                .aux_bytes
                .windows(3)
                .any(|window| window == b"MMZ")
        );
        assert!(
            records[0]
                .aux_bytes
                .windows(3)
                .any(|window| window == b"MLB")
        );
        assert!(
            records[0]
                .aux_bytes
                .windows(3)
                .any(|window| window == b"MNi")
        );
    }
}
