pub mod gzi;

mod gzip;
mod reader;
mod record;
mod unmapped;
mod writer;

pub use gzip::resolved_threads;
pub use reader::{
    count_fastq_records, count_fastq_records_with_label, open_fastq_reader,
    open_fastq_reader_with_label, read_next_fastq_record,
};
pub use record::{FastqIdentityBasis, FastqRecord, FastqRecordView};
pub use unmapped::{
    read_fastq_as_unmapped_records, read_fastq_as_unmapped_records_threaded_with_label,
    read_fastq_as_unmapped_records_with_label,
};
pub use writer::{FastqWriter, write_fastq_records};

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
