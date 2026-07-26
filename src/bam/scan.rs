use std::path::{Path, PathBuf};

use crate::{
    bam::{
        header::{HeaderPayload, parse_bam_header_from_reader},
        reader::BamReader,
        record::BamRecordView,
    },
    bgzf::virtual_offset::VirtualOffset,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BamRecordVirtualOffsets {
    pub start: VirtualOffset,
    pub end: VirtualOffset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryBamRecord {
    pub ref_id: i32,
    pub flags: u16,
    pub mapping_quality: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveSummaryBamRecord {
    pub summary: SummaryBamRecord,
    pub sequence_len: usize,
    pub quality_sum: u64,
    pub quality_count: u64,
}

#[derive(Debug, Clone)]
pub struct PositionedBamRecord<'a> {
    pub record: BamRecordView<'a>,
    pub virtual_offsets: BamRecordVirtualOffsets,
}

#[derive(Debug, Clone)]
pub struct PositionedRawBamRecord {
    pub raw_record: Vec<u8>,
    pub virtual_offsets: BamRecordVirtualOffsets,
}

impl BamScanner {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Self::open_with_label(path, path)
    }

    pub fn open_with_label(path: &Path, label: &Path) -> Result<Self, AppError> {
        let mut reader = BamReader::open_native_bgzf(path)?;
        let header = parse_bam_header_from_reader(&mut reader)?;

        Ok(Self {
            path: label.to_path_buf(),
            reader,
            header,
            raw_record: Vec::new(),
            records_read: 0,
        })
    }

    pub fn open_with_raw_sha256(path: &Path) -> Result<Self, AppError> {
        let mut reader = BamReader::open_native_bgzf_with_raw_sha256(path)?;
        let header = parse_bam_header_from_reader(&mut reader)?;
        Ok(Self {
            path: path.to_path_buf(),
            reader,
            header,
            raw_record: Vec::new(),
            records_read: 0,
        })
    }

    pub fn raw_sha256(&self) -> Option<&str> {
        self.reader.raw_sha256()
    }

    pub fn header(&self) -> &HeaderPayload {
        &self.header
    }

    pub fn records_read(&self) -> u64 {
        self.records_read
    }

    pub fn next_record(&mut self) -> Result<Option<BamRecordView<'_>>, AppError> {
        Ok(self
            .next_record_with_virtual_offsets()?
            .map(|positioned| positioned.record))
    }

    pub fn next_summary_record(&mut self) -> Result<Option<SummaryBamRecord>, AppError> {
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

        let block_size = block_size as usize;
        let mut core = [0_u8; BAM_CORE_SIZE as usize];
        self.reader.read_exact_into_with_context(
            &mut core,
            "BAM stream ended while reading an alignment record core section.",
        )?;
        let ref_id = read_i32_le(&core, 0, &self.path)?;
        let bin_mq_nl = read_u32_le(&core, 8, &self.path)?;
        let flag_nc = read_u32_le(&core, 12, &self.path)?;
        let sequence_len = read_i32_le(&core, 16, &self.path)?;

        if sequence_len < 0 {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record sequence length was negative.".to_string(),
            });
        }

        let remaining = block_size - BAM_CORE_SIZE as usize;
        let l_read_name = (bin_mq_nl & 0xff) as usize;
        if l_read_name == 0 {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record read name length was zero.".to_string(),
            });
        }

        let n_cigar_op = (flag_nc & 0xffff) as usize;
        let cigar_bytes = n_cigar_op
            .checked_mul(4)
            .ok_or_else(|| AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record CIGAR byte count overflowed usize.".to_string(),
            })?;
        let sequence_len = sequence_len as usize;
        let sequence_bytes = sequence_len.div_ceil(2);
        let quality_bytes = sequence_len;
        let consumed_after_core = l_read_name
            .checked_add(cigar_bytes)
            .and_then(|value| value.checked_add(sequence_bytes))
            .and_then(|value| value.checked_add(quality_bytes))
            .ok_or_else(|| AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record variable-length section overflowed usize.".to_string(),
            })?;

        if consumed_after_core > remaining {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: format!(
                    "BAM record declared block size {block_size} but needs at least {consumed_after_core} bytes after the core section."
                ),
            });
        }

        let read_name_bytes = self.reader.read_exact_vec_with_context(
            l_read_name,
            "BAM stream ended while reading an alignment record read name.",
        )?;
        let Some((&0, read_name_without_nul)) = read_name_bytes.split_last() else {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record read name was not NUL-terminated.".to_string(),
            });
        };
        std::str::from_utf8(read_name_without_nul).map_err(|error| AppError::InvalidRecord {
            path: self.path.clone(),
            detail: format!("BAM record read name is not valid UTF-8: {error}"),
        })?;

        self.reader.discard_exact_with_context(
            remaining - l_read_name,
            "BAM stream ended while skipping an alignment record variable payload.",
        )?;
        self.records_read += 1;

        Ok(Some(SummaryBamRecord {
            ref_id,
            flags: (flag_nc >> 16) as u16,
            mapping_quality: ((bin_mq_nl >> 8) & 0xff) as u8,
        }))
    }

    pub fn next_live_summary_record(&mut self) -> Result<Option<LiveSummaryBamRecord>, AppError> {
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

        let block_size = block_size as usize;
        let mut core = [0_u8; BAM_CORE_SIZE as usize];
        self.reader.read_exact_into_with_context(
            &mut core,
            "BAM stream ended while reading an alignment record core section.",
        )?;
        let ref_id = read_i32_le(&core, 0, &self.path)?;
        let bin_mq_nl = read_u32_le(&core, 8, &self.path)?;
        let flag_nc = read_u32_le(&core, 12, &self.path)?;
        let sequence_len = read_i32_le(&core, 16, &self.path)?;

        if sequence_len < 0 {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record sequence length was negative.".to_string(),
            });
        }

        let remaining = block_size - BAM_CORE_SIZE as usize;
        let l_read_name = (bin_mq_nl & 0xff) as usize;
        if l_read_name == 0 {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record read name length was zero.".to_string(),
            });
        }

        let n_cigar_op = (flag_nc & 0xffff) as usize;
        let cigar_bytes = n_cigar_op
            .checked_mul(4)
            .ok_or_else(|| AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record CIGAR byte count overflowed usize.".to_string(),
            })?;
        let sequence_len = sequence_len as usize;
        let sequence_bytes = sequence_len.div_ceil(2);
        let quality_bytes = sequence_len;
        let consumed_after_core = l_read_name
            .checked_add(cigar_bytes)
            .and_then(|value| value.checked_add(sequence_bytes))
            .and_then(|value| value.checked_add(quality_bytes))
            .ok_or_else(|| AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record variable-length section overflowed usize.".to_string(),
            })?;

        if consumed_after_core > remaining {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: format!(
                    "BAM record declared block size {block_size} but needs at least {consumed_after_core} bytes after the core section."
                ),
            });
        }

        let read_name_bytes = self.reader.read_exact_vec_with_context(
            l_read_name,
            "BAM stream ended while reading an alignment record read name.",
        )?;
        let Some((&0, read_name_without_nul)) = read_name_bytes.split_last() else {
            return Err(AppError::InvalidRecord {
                path: self.path.clone(),
                detail: "BAM record read name was not NUL-terminated.".to_string(),
            });
        };
        std::str::from_utf8(read_name_without_nul).map_err(|error| AppError::InvalidRecord {
            path: self.path.clone(),
            detail: format!("BAM record read name is not valid UTF-8: {error}"),
        })?;

        self.reader.discard_exact_with_context(
            cigar_bytes + sequence_bytes,
            "BAM stream ended while skipping alignment record CIGAR and sequence payload.",
        )?;
        let mut quality_sum = 0_u64;
        let mut quality_count = 0_u64;
        let mut remaining_quality_bytes = quality_bytes;
        let mut quality_buffer = [0_u8; 8192];
        while remaining_quality_bytes > 0 {
            let chunk_len = remaining_quality_bytes.min(quality_buffer.len());
            self.reader.read_exact_into_with_context(
                &mut quality_buffer[..chunk_len],
                "BAM stream ended while reading alignment record quality scores.",
            )?;
            for &score in &quality_buffer[..chunk_len] {
                if score != 0xff {
                    quality_sum += u64::from(score);
                    quality_count += 1;
                }
            }
            remaining_quality_bytes -= chunk_len;
        }
        self.reader.discard_exact_with_context(
            remaining - consumed_after_core,
            "BAM stream ended while skipping an alignment record auxiliary payload.",
        )?;
        self.records_read += 1;

        Ok(Some(LiveSummaryBamRecord {
            summary: SummaryBamRecord {
                ref_id,
                flags: (flag_nc >> 16) as u16,
                mapping_quality: ((bin_mq_nl >> 8) & 0xff) as u8,
            },
            sequence_len,
            quality_sum,
            quality_count,
        }))
    }

    pub fn next_record_with_virtual_offsets(
        &mut self,
    ) -> Result<Option<PositionedBamRecord<'_>>, AppError> {
        let Some(virtual_offsets) = self.read_record_bytes_with_virtual_offsets()? else {
            return Ok(None);
        };

        let view =
            BamRecordView::parse(&self.raw_record).map_err(|error| AppError::InvalidRecord {
                path: self.path.clone(),
                detail: error.detail().to_string(),
            })?;
        self.records_read += 1;
        Ok(Some(PositionedBamRecord {
            record: view,
            virtual_offsets,
        }))
    }

    pub fn next_raw_record_with_virtual_offsets(
        &mut self,
    ) -> Result<Option<PositionedRawBamRecord>, AppError> {
        let Some(virtual_offsets) = self.read_record_bytes_with_virtual_offsets()? else {
            return Ok(None);
        };

        BamRecordView::parse(&self.raw_record).map_err(|error| AppError::InvalidRecord {
            path: self.path.clone(),
            detail: error.detail().to_string(),
        })?;
        self.records_read += 1;
        Ok(Some(PositionedRawBamRecord {
            raw_record: self.raw_record.clone(),
            virtual_offsets,
        }))
    }

    pub fn seek_virtual_offset(&mut self, offset: VirtualOffset) -> Result<(), AppError> {
        self.reader.seek_virtual_offset(offset)
    }

    pub fn raw_records_in_virtual_range(
        &mut self,
        start: VirtualOffset,
        end: VirtualOffset,
    ) -> Result<Vec<PositionedRawBamRecord>, AppError> {
        if start >= end {
            return Err(AppError::InvalidIndex {
                path: self.path.clone(),
                detail: format!(
                    "Virtual-offset range start {} must be before end {}.",
                    start.packed(),
                    end.packed()
                ),
            });
        }

        self.seek_virtual_offset(start)?;
        let mut records = Vec::new();
        while let Some(record) = self.next_raw_record_with_virtual_offsets()? {
            if record.virtual_offsets.start >= end {
                break;
            }
            if record.virtual_offsets.end > start {
                records.push(record);
            }
        }
        Ok(records)
    }

    fn read_record_bytes_with_virtual_offsets(
        &mut self,
    ) -> Result<Option<BamRecordVirtualOffsets>, AppError> {
        self.raw_record.clear();

        let record_start = self.current_virtual_offset()?;
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
        let record_end = self.current_virtual_offset()?;

        Ok(Some(BamRecordVirtualOffsets {
            start: record_start,
            end: record_end,
        }))
    }

    fn current_virtual_offset(&self) -> Result<VirtualOffset, AppError> {
        self.reader
            .virtual_offset()?
            .ok_or_else(|| AppError::InvalidBam {
                path: self.path.clone(),
                detail: "Native BAM scanner backend did not expose BGZF virtual offsets."
                    .to_string(),
            })
    }
}

fn read_i32_le(raw: &[u8], offset: usize, path: &Path) -> Result<i32, AppError> {
    let bytes = read_4(raw, offset, path)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u32_le(raw: &[u8], offset: usize, path: &Path) -> Result<u32, AppError> {
    let bytes = read_4(raw, offset, path)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_4(raw: &[u8], offset: usize, path: &Path) -> Result<[u8; 4], AppError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "BAM record fixed-field offset overflowed usize.".to_string(),
        })?;
    let bytes = raw
        .get(offset..end)
        .ok_or_else(|| AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "BAM record ended before the core fields were available.".to_string(),
        })?;
    Ok(bytes.try_into().expect("slice length checked above"))
}

#[cfg(test)]
mod tests {
    use crate::{
        bam::index::build_bai_index_from_bam,
        bam::record::BamRecordView,
        bam::scan::BamScanner,
        bgzf::BGZF_EOF_MARKER,
        bgzf::test_support::{
            build_bam_file_with_header_and_records, build_bgzf_member, build_light_record,
            write_temp_file,
        },
        bgzf::virtual_offset::VirtualOffset,
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
    fn scans_summary_records_without_materializing_full_views() {
        let first = build_light_record(0, 5, "read1", 0x41);
        let second = build_light_record(-1, -1, "read2", 0x4 | 0x80);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[first, second],
        );
        let path = write_temp_file("scanner-summary-records", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let first = scanner
            .next_summary_record()
            .expect("scan should succeed")
            .expect("first record should exist");
        let second = scanner
            .next_summary_record()
            .expect("scan should succeed")
            .expect("second record should exist");

        assert_eq!((first.ref_id, first.flags), (0, 0x41));
        assert_eq!((second.ref_id, second.flags), (-1, 0x4 | 0x80));
        assert!(
            scanner
                .next_summary_record()
                .expect("scan should succeed")
                .is_none()
        );
        assert_eq!(scanner.records_read(), 2);
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn scans_live_summary_records_with_length_and_quality_sums() {
        let mut variable = Vec::new();
        variable.extend_from_slice(b"read1\0");
        variable.extend_from_slice(&[0x12, 0x34]);
        variable.extend_from_slice(&[10, 20, 30, 0xff]);
        variable.extend_from_slice(b"NMc\x05");
        let mut record = Vec::new();
        let mut payload = minimal_record_payload(0, 5, 0, 4, 6, &variable);
        record.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        record.append(&mut payload);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record],
        );
        let path = write_temp_file("scanner-live-summary-records", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let record = scanner
            .next_live_summary_record()
            .expect("scan should succeed")
            .expect("record should exist");

        assert_eq!(record.summary.ref_id, 0);
        assert_eq!(record.sequence_len, 4);
        assert_eq!(record.quality_sum, 60);
        assert_eq!(record.quality_count, 3);
        assert!(
            scanner
                .next_live_summary_record()
                .expect("scan should succeed")
                .is_none()
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn reports_single_block_record_virtual_offsets() {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(0, 8, "read2", 0);
        let header_text = "@SQ\tSN:chr1\tLN:10\n";
        let payload = build_bam_payload(
            header_text,
            &[("chr1", 10)],
            &[first.clone(), second.clone()],
        );
        let first_member = build_bgzf_member(&payload);
        let mut bytes = first_member.clone();
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("scanner-offsets-single-block", "bam", &bytes);

        let expected_first_start = payload.len() - first.len() - second.len();
        let expected_second_start = expected_first_start + first.len();
        let mut scanner = BamScanner::open(&path).expect("scanner should open");

        let positioned_first = scanner
            .next_record_with_virtual_offsets()
            .expect("scan should succeed")
            .expect("first record should exist");
        assert_eq!(positioned_first.record.read_name(), "read1");
        assert_eq!(
            positioned_first
                .virtual_offsets
                .start
                .compressed_block_offset(),
            0
        );
        assert_eq!(
            positioned_first
                .virtual_offsets
                .start
                .uncompressed_block_offset(),
            expected_first_start as u16
        );
        assert_eq!(
            positioned_first
                .virtual_offsets
                .end
                .compressed_block_offset(),
            0
        );
        assert_eq!(
            positioned_first
                .virtual_offsets
                .end
                .uncompressed_block_offset(),
            (expected_first_start + first.len()) as u16
        );

        let positioned_second = scanner
            .next_record_with_virtual_offsets()
            .expect("scan should succeed")
            .expect("second record should exist");
        assert_eq!(positioned_second.record.read_name(), "read2");
        assert_eq!(
            positioned_second
                .virtual_offsets
                .start
                .compressed_block_offset(),
            0
        );
        assert_eq!(
            positioned_second
                .virtual_offsets
                .start
                .uncompressed_block_offset(),
            expected_second_start as u16
        );
        assert_eq!(
            positioned_second
                .virtual_offsets
                .end
                .compressed_block_offset(),
            first_member.len() as u64
        );
        assert_eq!(
            positioned_second
                .virtual_offsets
                .end
                .uncompressed_block_offset(),
            0
        );

        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn reports_record_virtual_offsets_across_bgzf_members() {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(0, 8, "read2", 0);
        let header_payload = build_bam_payload("@SQ\tSN:chr1\tLN:10\n", &[("chr1", 10)], &[]);
        let mut record_payload = Vec::new();
        record_payload.extend_from_slice(&first);
        record_payload.extend_from_slice(&second);
        let first_member = build_bgzf_member(&header_payload);
        let second_member = build_bgzf_member(&record_payload);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first_member);
        bytes.extend_from_slice(&second_member);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("scanner-offsets-multi-block", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let positioned_first = scanner
            .next_record_with_virtual_offsets()
            .expect("scan should succeed")
            .expect("first record should exist");
        assert_eq!(positioned_first.record.read_name(), "read1");
        assert_eq!(
            positioned_first
                .virtual_offsets
                .start
                .compressed_block_offset(),
            first_member.len() as u64
        );
        assert_eq!(
            positioned_first
                .virtual_offsets
                .start
                .uncompressed_block_offset(),
            0
        );
        assert_eq!(
            positioned_first
                .virtual_offsets
                .end
                .compressed_block_offset(),
            first_member.len() as u64
        );
        assert_eq!(
            positioned_first
                .virtual_offsets
                .end
                .uncompressed_block_offset(),
            first.len() as u16
        );

        let positioned_second = scanner
            .next_record_with_virtual_offsets()
            .expect("scan should succeed")
            .expect("second record should exist");
        assert_eq!(positioned_second.record.read_name(), "read2");
        assert_eq!(
            positioned_second
                .virtual_offsets
                .start
                .compressed_block_offset(),
            first_member.len() as u64
        );
        assert_eq!(
            positioned_second
                .virtual_offsets
                .start
                .uncompressed_block_offset(),
            first.len() as u16
        );
        assert_eq!(
            positioned_second
                .virtual_offsets
                .end
                .compressed_block_offset(),
            first_member.len() as u64 + second_member.len() as u64
        );
        assert_eq!(
            positioned_second
                .virtual_offsets
                .end
                .uncompressed_block_offset(),
            0
        );

        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn fetches_records_from_indexed_virtual_range() {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(0, 8, "read2", 0);
        let third = build_light_record(0, 12, "read3", 0);
        let header_payload = build_bam_payload("@SQ\tSN:chr1\tLN:20\n", &[("chr1", 20)], &[]);
        let mut first_record_payload = Vec::new();
        first_record_payload.extend_from_slice(&first);
        first_record_payload.extend_from_slice(&second);
        let second_record_payload = third.clone();
        let header_member = build_bgzf_member(&header_payload);
        let first_record_member = build_bgzf_member(&first_record_payload);
        let second_record_member = build_bgzf_member(&second_record_payload);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&header_member);
        bytes.extend_from_slice(&first_record_member);
        bytes.extend_from_slice(&second_record_member);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("scanner-indexed-range", "bam", &bytes);
        let start =
            VirtualOffset::new(header_member.len() as u64, 0).expect("start offset should fit");
        let end = VirtualOffset::new(
            header_member.len() as u64 + first_record_member.len() as u64,
            0,
        )
        .expect("end offset should fit");

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let records = scanner
            .raw_records_in_virtual_range(start, end)
            .expect("indexed range should read");
        let names = records
            .iter()
            .map(|record| {
                BamRecordView::parse(&record.raw_record)
                    .expect("raw record should parse")
                    .read_name()
                    .to_string()
            })
            .collect::<Vec<_>>();

        std::fs::remove_file(path).expect("fixture should be removable");
        assert_eq!(names, vec!["read1", "read2"]);
    }

    #[test]
    fn fetches_records_from_native_bai_chunk() {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(0, 8, "read2", 0);
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:20\n",
            &[("chr1", 20)],
            &[first, second],
        );
        let path = write_temp_file("scanner-bai-chunk-range", "bam", &bytes);
        let index = build_bai_index_from_bam(&path).expect("bai index should build");
        let chunk = index.references[0]
            .bins
            .values()
            .flat_map(|chunks| chunks.iter())
            .next()
            .expect("chunk should exist")
            .clone();

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let records = scanner
            .raw_records_in_virtual_range(chunk.start, chunk.end)
            .expect("bai chunk should read");
        let names = records
            .iter()
            .map(|record| {
                BamRecordView::parse(&record.raw_record)
                    .expect("raw record should parse")
                    .read_name()
                    .to_string()
            })
            .collect::<Vec<_>>();

        std::fs::remove_file(path).expect("fixture should be removable");
        assert_eq!(names, vec!["read1", "read2"]);
    }

    #[test]
    fn rejects_empty_virtual_range() {
        let bytes =
            build_bam_file_with_header_and_records("@SQ\tSN:chr1\tLN:10\n", &[("chr1", 10)], &[]);
        let path = write_temp_file("scanner-empty-range", "bam", &bytes);
        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let offset = VirtualOffset::ZERO;

        let error = scanner
            .raw_records_in_virtual_range(offset, offset)
            .expect_err("empty range should fail");

        std::fs::remove_file(path).expect("fixture should be removable");
        assert_eq!(error.to_json_error().code, "invalid_index");
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

    #[test]
    fn rejects_record_with_variable_sections_beyond_block_size() {
        let mut payload = minimal_record_payload(0, 1, 0, 0, 8, b"read1\0");
        let block_size = payload.len() as i32;
        let mut tail = Vec::new();
        tail.extend_from_slice(&block_size.to_le_bytes());
        tail.append(&mut payload);
        let bytes = build_bam_with_raw_tail(&tail);
        let path = write_temp_file("scanner-variable-overflow", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let error = scanner
            .next_record()
            .expect_err("variable section overflow should be rejected");

        let json_error = error.to_json_error();
        assert_eq!(json_error.code, "invalid_record");
        assert!(
            json_error
                .detail
                .is_some_and(|detail| detail.contains("needs at least"))
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn rejects_record_with_unterminated_read_name() {
        let mut payload = minimal_record_payload(0, 1, 0, 0, 6, b"read1!");
        let block_size = payload.len() as i32;
        let mut tail = Vec::new();
        tail.extend_from_slice(&block_size.to_le_bytes());
        tail.append(&mut payload);
        let bytes = build_bam_with_raw_tail(&tail);
        let path = write_temp_file("scanner-unterminated-read-name", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let error = scanner
            .next_record()
            .expect_err("unterminated read name should be rejected");

        let json_error = error.to_json_error();
        assert_eq!(json_error.code, "invalid_record");
        assert!(
            json_error
                .detail
                .is_some_and(|detail| detail.contains("NUL-terminated"))
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn rejects_record_with_negative_sequence_length() {
        let mut payload = minimal_record_payload(0, 1, 0, -1, 6, b"read1\0");
        let block_size = payload.len() as i32;
        let mut tail = Vec::new();
        tail.extend_from_slice(&block_size.to_le_bytes());
        tail.append(&mut payload);
        let bytes = build_bam_with_raw_tail(&tail);
        let path = write_temp_file("scanner-negative-sequence-length", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let error = scanner
            .next_record()
            .expect_err("negative sequence length should be rejected");

        let json_error = error.to_json_error();
        assert_eq!(json_error.code, "invalid_record");
        assert!(
            json_error
                .detail
                .is_some_and(|detail| detail.contains("sequence length"))
        );
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn scanner_view_matches_owned_layout_for_valid_record() {
        let record = build_light_record(0, 7, "read1", 0x10);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record],
        );
        let path = write_temp_file("scanner-layout-differential", "bam", &bytes);

        let mut scanner = BamScanner::open(&path).expect("scanner should open");
        let view = scanner
            .next_record()
            .expect("scanner should succeed")
            .expect("record should be present");
        let layout = view.to_record_layout();

        assert_eq!(view.block_size(), layout.block_size);
        assert_eq!(view.ref_id(), layout.ref_id);
        assert_eq!(view.pos(), layout.pos);
        assert_eq!(view.flags(), layout.flags);
        assert_eq!(view.mapping_quality(), layout.mapping_quality);
        assert_eq!(view.read_name(), layout.read_name);
        assert_eq!(view.cigar_bytes(), layout.cigar_bytes);
        assert_eq!(view.sequence_bytes(), layout.sequence_bytes);
        assert_eq!(view.quality_bytes(), layout.quality_bytes);
        assert_eq!(view.aux_bytes(), layout.aux_bytes);
        std::fs::remove_file(path).expect("fixture should be removable");
    }

    fn minimal_record_payload(
        ref_id: i32,
        pos: i32,
        flags: u16,
        sequence_len: i32,
        read_name_len: u32,
        variable: &[u8],
    ) -> Vec<u8> {
        let bin_mq_nl = read_name_len;
        let flag_nc = (flags as u32) << 16;
        let mut payload = Vec::with_capacity(32 + variable.len());
        payload.extend_from_slice(&ref_id.to_le_bytes());
        payload.extend_from_slice(&pos.to_le_bytes());
        payload.extend_from_slice(&bin_mq_nl.to_le_bytes());
        payload.extend_from_slice(&flag_nc.to_le_bytes());
        payload.extend_from_slice(&sequence_len.to_le_bytes());
        payload.extend_from_slice(&(-1_i32).to_le_bytes());
        payload.extend_from_slice(&(-1_i32).to_le_bytes());
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(variable);
        payload
    }

    fn build_bam_payload(
        header_text: &str,
        references: &[(&str, u32)],
        records: &[Vec<u8>],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&(header_text.len() as i32).to_le_bytes());
        payload.extend_from_slice(header_text.as_bytes());
        payload.extend_from_slice(&(references.len() as i32).to_le_bytes());

        for (name, length) in references {
            let mut nul_terminated_name = name.as_bytes().to_vec();
            nul_terminated_name.push(0);
            payload.extend_from_slice(&(nul_terminated_name.len() as i32).to_le_bytes());
            payload.extend_from_slice(&nul_terminated_name);
            payload.extend_from_slice(&(*length as i32).to_le_bytes());
        }

        for record in records {
            payload.extend_from_slice(record);
        }

        payload
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
