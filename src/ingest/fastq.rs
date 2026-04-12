use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use flate2::read::MultiGzDecoder;

use crate::{
    bam::records::{
        BAM_FUNMAP, RecordLayout, encode_bam_qualities, encode_bam_sequence, missing_quality_scores,
    },
    error::AppError,
};

pub fn read_fastq_as_unmapped_records(
    path: &Path,
    read_group: Option<&str>,
) -> Result<Vec<RecordLayout>, AppError> {
    let mut reader = open_fastq_reader(path)?;
    let mut records = Vec::new();

    loop {
        let Some(header_line) = read_next_line(&mut reader, path)? else {
            break;
        };
        let sequence_line = required_line(&mut reader, path, "sequence")?;
        let plus_line = required_line(&mut reader, path, "plus")?;
        let quality_line = required_line(&mut reader, path, "quality")?;

        records.push(build_unmapped_record(
            path,
            &header_line,
            &sequence_line,
            &plus_line,
            &quality_line,
            read_group,
        )?);
    }

    Ok(records)
}

fn open_fastq_reader(path: &Path) -> Result<Box<dyn BufRead>, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
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
    plus_line: &str,
    quality_line: &str,
    read_group: Option<&str>,
) -> Result<RecordLayout, AppError> {
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

    let aux_bytes = read_group.map_or_else(Vec::new, encode_read_group_aux);
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

    use super::read_fastq_as_unmapped_records;

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
}
