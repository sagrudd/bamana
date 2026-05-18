use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    thread,
};

use flate2::read::MultiGzDecoder;

use crate::error::AppError;

pub(crate) fn open_maybe_gzip_reader(
    path: &Path,
    label: &Path,
) -> Result<Box<dyn BufRead>, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(label, error))?;
    if is_gzip_fastq_path(path) {
        Ok(Box::new(BufReader::new(MultiGzDecoder::new(file))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

pub fn is_gzip_fastq_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
}

pub fn resolved_threads(requested_threads: usize) -> usize {
    let available = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1);
    if requested_threads == 0 {
        available.max(1)
    } else {
        requested_threads.min(available).max(1)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
    };

    use flate2::{Compression, write::GzEncoder};

    use crate::{
        error::AppError,
        fastq::{
            count_fastq_records,
            gzi::{build_fastq_gzi, fastq_gzi_output_path},
            open_fastq_reader, read_next_fastq_record,
        },
    };

    use super::is_gzip_fastq_path;

    fn temp_path(stem: &str, extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!("bamana-{stem}-{}-{extension}", std::process::id()))
    }

    fn gzip_member(payload: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(payload)
            .expect("gzip member should write");
        encoder.finish().expect("gzip member should finish")
    }

    fn write_bytes(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).expect("fixture should write");
    }

    fn remove_if_exists(path: &Path) {
        if path.exists() {
            fs::remove_file(path).expect("fixture should be removable");
        }
    }

    #[test]
    fn gzip_detection_is_extension_based_and_case_insensitive() {
        assert!(is_gzip_fastq_path(Path::new("reads.fastq.gz")));
        assert!(is_gzip_fastq_path(Path::new("reads.FQ.GZ")));
        assert!(!is_gzip_fastq_path(Path::new("reads.fastq")));
        assert!(!is_gzip_fastq_path(Path::new("reads.gz.tmp")));
    }

    #[test]
    fn reads_valid_single_member_fastq_gz() {
        let path = temp_path("single-member", "fastq.gz");
        write_bytes(&path, &gzip_member(b"@read1\nACGT\n+\n!!!!\n"));

        let count = count_fastq_records(&path).expect("single-member gzip should count");
        let mut reader = open_fastq_reader(&path).expect("single-member gzip should open");
        let record = read_next_fastq_record(&mut reader, &path)
            .expect("single-member gzip should parse")
            .expect("record should exist");
        let eof = read_next_fastq_record(&mut reader, &path).expect("EOF should be clean");
        remove_if_exists(&path);

        assert_eq!(count, 1);
        assert_eq!(record.read_name, "read1");
        assert_eq!(record.sequence, "ACGT");
        assert_eq!(record.quality, "!!!!");
        assert!(eof.is_none());
    }

    #[test]
    fn reads_valid_multi_member_fastq_gz_in_member_order() {
        let path = temp_path("multi-member", "fastq.gz");
        let mut bytes = gzip_member(b"@read1\nAC\n+\n!!\n");
        bytes.extend_from_slice(&gzip_member(b"@read2\nTGCA\n+two\n####\n"));
        write_bytes(&path, &bytes);

        let count = count_fastq_records(&path).expect("multi-member gzip should count");
        let mut reader = open_fastq_reader(&path).expect("multi-member gzip should open");
        let first = read_next_fastq_record(&mut reader, &path)
            .expect("first record should parse")
            .expect("first record should exist");
        let second = read_next_fastq_record(&mut reader, &path)
            .expect("second record should parse")
            .expect("second record should exist");
        let eof = read_next_fastq_record(&mut reader, &path).expect("EOF should be clean");
        remove_if_exists(&path);

        assert_eq!(count, 2);
        assert_eq!(first.read_name, "read1");
        assert_eq!(first.sequence, "AC");
        assert_eq!(second.read_name, "read2");
        assert_eq!(second.plus_line, "+two");
        assert_eq!(second.quality, "####");
        assert!(eof.is_none());
    }

    #[test]
    fn corrupt_gzip_stream_reports_structured_io_error() {
        let path = temp_path("corrupt", "fastq.gz");
        write_bytes(&path, b"not a gzip stream");

        let error = count_fastq_records(&path).expect_err("corrupt gzip should fail");
        remove_if_exists(&path);

        match error {
            AppError::Io { path, message } => {
                assert_eq!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some(format!("bamana-corrupt-{}-fastq.gz", std::process::id()).as_str())
                );
                assert!(!message.is_empty());
            }
            other => panic!("expected structured Io error, got {other:?}"),
        }
    }

    #[test]
    fn truncated_gzip_stream_reports_structured_io_error() {
        let path = temp_path("truncated", "fastq.gz");
        let mut bytes = gzip_member(b"@read1\nACGT\n+\n!!!!\n");
        bytes.truncate(bytes.len().saturating_sub(4));
        write_bytes(&path, &bytes);

        let error = count_fastq_records(&path).expect_err("truncated gzip should fail");
        remove_if_exists(&path);

        match error {
            AppError::Io { path, message } => {
                assert_eq!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some(format!("bamana-truncated-{}-fastq.gz", std::process::id()).as_str())
                );
                assert!(!message.is_empty());
            }
            other => panic!("expected structured Io error, got {other:?}"),
        }
    }

    #[test]
    fn fastq_gzi_builds_for_multi_member_fastq_gz() {
        let path = temp_path("multi-member-gzi", "fastq.gz");
        let index_path = fastq_gzi_output_path(&path);
        let mut bytes = gzip_member(b"@read1\nAC\n+\n!!\n");
        bytes.extend_from_slice(&gzip_member(b"@read2\nTG\n+\n##\n"));
        write_bytes(&path, &bytes);

        let summary = build_fastq_gzi(&path, &index_path).expect("FASTQ.GZI should build");
        remove_if_exists(&index_path);
        remove_if_exists(&path);

        assert_eq!(summary.total_records, 2);
        assert!(
            summary.checkpoints.len() >= 2,
            "start and end checkpoints should be present"
        );
    }
}
