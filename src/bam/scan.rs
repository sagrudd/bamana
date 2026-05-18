use std::path::{Path, PathBuf};

use crate::{
    bam::{
        header::{HeaderPayload, parse_bam_header_from_reader},
        reader::BamReader,
        record::BamRecordView,
    },
    error::AppError,
};

const BAM_CORE_SIZE: i32 = 32;

pub struct BamScanner {
    path: PathBuf,
    reader: BamReader,
    header: HeaderPayload,
    raw_record: Vec<u8>,
    records_read: u64,
}

impl BamScanner {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let mut reader = BamReader::open_native_bgzf(path)?;
        let header = parse_bam_header_from_reader(&mut reader)?;

        Ok(Self {
            path: path.to_path_buf(),
            reader,
            header,
            raw_record: Vec::new(),
            records_read: 0,
        })
    }

    pub fn header(&self) -> &HeaderPayload {
        &self.header
    }

    pub fn records_read(&self) -> u64 {
        self.records_read
    }

    pub fn next_record(&mut self) -> Result<Option<BamRecordView<'_>>, AppError> {
        self.raw_record.clear();

        let Some(block_size) = self.reader.read_optional_i32_le()? else {
            return Ok(None);
        };

        if block_size < 0 {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: format!("BAM record block size {block_size} was negative."),
            });
        }

        if block_size < BAM_CORE_SIZE {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: format!(
                    "BAM record block size {block_size} is smaller than the 32-byte core alignment section."
                ),
            });
        }

        self.raw_record.extend_from_slice(&block_size.to_le_bytes());
        let payload = self.reader.read_exact_vec_with_context(
            block_size as usize,
            "BAM stream ended while reading an alignment record payload.",
        )?;
        self.raw_record.extend_from_slice(&payload);

        let view =
            BamRecordView::parse(&self.raw_record).map_err(|error| AppError::InvalidRecord {
                path: self.path.clone(),
                detail: error.detail().to_string(),
            })?;
        self.records_read += 1;
        Ok(Some(view))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bam::scan::BamScanner,
        bgzf::test_support::{
            build_bam_file_with_header_and_records, build_bgzf_member, build_light_record,
            write_temp_file,
        },
    };

    #[test]
    fn scans_empty_bam_body_after_native_header() {
        let bytes =
            build_bam_file_with_header_and_records("@SQ\tSN:chr1\tLN:10\n", &[("chr1", 10)], &[]);
        let path = write_temp_file("scanner-empty", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");

        assert_eq!(scanner.header().header.references.len(), 1);
        assert!(
            scanner
                .next_record()
                .expect("scan should succeed")
                .is_none()
        );
        assert_eq!(scanner.records_read(), 0);
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn scans_single_record_in_encounter_order() {
        let record = build_light_record(0, 5, "read1", 0x10);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record],
        );
        let path = write_temp_file("scanner-single", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let record = scanner
            .next_record()
            .expect("scan should succeed")
            .expect("record should exist");

        assert_eq!(record.ref_id(), 0);
        assert_eq!(record.pos(), 5);
        assert_eq!(record.flags(), 0x10);
        assert_eq!(record.read_name(), "read1");
        assert!(
            scanner
                .next_record()
                .expect("scan should succeed")
                .is_none()
        );
        assert_eq!(scanner.records_read(), 1);
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn scans_multiple_records_in_encounter_order() {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(1, 9, "read2", 0x4);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n@SQ\tSN:chr2\tLN:20\n",
            &[("chr1", 10), ("chr2", 20)],
            &[first, second],
        );
        let path = write_temp_file("scanner-multi", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let first = scanner
            .next_record()
            .expect("scan should succeed")
            .expect("first record should exist");
        assert_eq!(
            (first.ref_id(), first.pos(), first.read_name()),
            (0, 5, "read1")
        );

        let second = scanner
            .next_record()
            .expect("scan should succeed")
            .expect("second record should exist");
        assert_eq!(
            (
                second.ref_id(),
                second.pos(),
                second.read_name(),
                second.flags()
            ),
            (1, 9, "read2", 0x4)
        );

        assert!(
            scanner
                .next_record()
                .expect("scan should succeed")
                .is_none()
        );
        assert_eq!(scanner.records_read(), 2);
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn rejects_negative_record_block_size() {
        let bytes = build_bam_with_raw_tail(&(-1_i32).to_le_bytes());
        let path = write_temp_file("scanner-negative-size", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let error = scanner
            .next_record()
            .expect_err("negative size should be rejected");

        let json_error = error.to_json_error();
        assert_eq!(json_error.code, "invalid_record");
        assert!(
            json_error
                .detail
                .is_some_and(|detail| detail.contains("negative"))
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn rejects_record_block_size_smaller_than_core() {
        let bytes = build_bam_with_raw_tail(&31_i32.to_le_bytes());
        let path = write_temp_file("scanner-small-size", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let error = scanner
            .next_record()
            .expect_err("small size should be rejected");

        assert!(
            error
                .to_json_error()
                .detail
                .is_some_and(|detail| detail.contains("smaller"))
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn reports_truncated_record_payload() {
        let mut tail = Vec::new();
        tail.extend_from_slice(&36_i32.to_le_bytes());
        tail.extend_from_slice(&[0_u8; 8]);
        let bytes = build_bam_with_raw_tail(&tail);
        let path = write_temp_file("scanner-truncated-payload", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let error = scanner
            .next_record()
            .expect_err("truncated payload should be rejected");

        let json_error = error.to_json_error();
        assert_eq!(json_error.code, "truncated_file");
        assert!(
            json_error
                .detail
                .is_some_and(|detail| detail.contains("payload"))
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn reports_truncated_record_block_size() {
        let bytes = build_bam_with_raw_tail(&[0x24, 0x00]);
        let path = write_temp_file("scanner-truncated-size", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let error = scanner
            .next_record()
            .expect_err("truncated block size should be rejected");

        let json_error = error.to_json_error();
        assert_eq!(json_error.code, "truncated_file");
        assert!(
            json_error
                .detail
                .is_some_and(|detail| detail.contains("block size"))
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    fn build_bam_with_raw_tail(tail: &[u8]) -> Vec<u8> {
        let header_text = "@SQ\tSN:chr1\tLN:10\n";
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&(header_text.len() as i32).to_le_bytes());
        payload.extend_from_slice(header_text.as_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&5_i32.to_le_bytes());
        payload.extend_from_slice(b"chr1\0");
        payload.extend_from_slice(&10_i32.to_le_bytes());
        payload.extend_from_slice(tail);

        let mut bytes = build_bgzf_member(&payload);
        bytes.extend_from_slice(&crate::bgzf::BGZF_EOF_MARKER);
        bytes
    }
}
