use std::{error::Error, fmt, ops::Range, str};

use crate::bam::records::RecordLayout;

const BLOCK_SIZE_PREFIX: usize = 4;
const BAM_CORE_SIZE: usize = 32;
const CORE_START: usize = BLOCK_SIZE_PREFIX;
const VARIABLE_START: usize = BLOCK_SIZE_PREFIX + BAM_CORE_SIZE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BamRecordSections {
    pub raw_record: Range<usize>,
    pub core: Range<usize>,
    pub read_name: Range<usize>,
    pub cigar: Range<usize>,
    pub sequence: Range<usize>,
    pub qualities: Range<usize>,
    pub aux: Range<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BamRecordFlags {
    pub raw: u16,
    pub is_paired: bool,
    pub is_proper_pair: bool,
    pub is_unmapped: bool,
    pub is_mate_unmapped: bool,
    pub is_reverse: bool,
    pub is_mate_reverse: bool,
    pub is_read1: bool,
    pub is_read2: bool,
    pub is_secondary: bool,
    pub is_qc_fail: bool,
    pub is_duplicate: bool,
    pub is_supplementary: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BamRecordCoordinates {
    pub ref_id: i32,
    pub pos: i32,
    pub next_ref_id: i32,
    pub next_pos: i32,
    pub template_len: i32,
    pub bin: u16,
}

/// Zero-based, half-open mapped reference interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BamReferenceInterval {
    pub reference_index: usize,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BamRecordSkipOffsets {
    pub after_core: usize,
    pub after_read_name: usize,
    pub after_cigar: usize,
    pub after_sequence: usize,
    pub after_qualities: usize,
    pub after_aux: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BamRecordView<'a> {
    raw_record: &'a [u8],
    sections: BamRecordSections,
    block_size: usize,
    ref_id: i32,
    pos: i32,
    bin: u16,
    next_ref_id: i32,
    next_pos: i32,
    template_len: i32,
    flags: u16,
    mapping_quality: u8,
    n_cigar_op: usize,
    sequence_len: usize,
    read_name: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BamRecordViewError {
    detail: String,
}

impl BamRecordViewError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for BamRecordViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for BamRecordViewError {}

impl<'a> BamRecordView<'a> {
    pub fn parse(raw_record: &'a [u8]) -> Result<Self, BamRecordViewError> {
        if raw_record.len() < BLOCK_SIZE_PREFIX {
            return Err(BamRecordViewError::new(
                "BAM record slice is shorter than the 4-byte block size prefix.",
            ));
        }

        let block_size = read_i32_le(raw_record, 0)?;
        if block_size < BAM_CORE_SIZE as i32 {
            return Err(BamRecordViewError::new(format!(
                "BAM record block size {block_size} is smaller than the 32-byte core alignment section."
            )));
        }
        let block_size = block_size as usize;
        let expected_len = BLOCK_SIZE_PREFIX.checked_add(block_size).ok_or_else(|| {
            BamRecordViewError::new(
                "BAM record block size overflowed usize when adding the prefix.",
            )
        })?;

        if raw_record.len() != expected_len {
            return Err(BamRecordViewError::new(format!(
                "BAM record declared block size {block_size} but supplied slice has {} bytes including the prefix.",
                raw_record.len()
            )));
        }

        let ref_id = read_i32_le(raw_record, 4)?;
        let pos = read_i32_le(raw_record, 8)?;
        let bin_mq_nl = read_u32_le(raw_record, 12)?;
        let flag_nc = read_u32_le(raw_record, 16)?;
        let sequence_len = read_i32_le(raw_record, 20)?;
        let next_ref_id = read_i32_le(raw_record, 24)?;
        let next_pos = read_i32_le(raw_record, 28)?;
        let template_len = read_i32_le(raw_record, 32)?;

        if sequence_len < 0 {
            return Err(BamRecordViewError::new(
                "BAM record sequence length was negative.",
            ));
        }

        let l_read_name = (bin_mq_nl & 0xff) as usize;
        if l_read_name == 0 {
            return Err(BamRecordViewError::new(
                "BAM record read name length was zero.",
            ));
        }

        let mapping_quality = ((bin_mq_nl >> 8) & 0xff) as u8;
        let bin = (bin_mq_nl >> 16) as u16;
        let n_cigar_op = (flag_nc & 0xffff) as usize;
        let flags = (flag_nc >> 16) as u16;
        let sequence_len = sequence_len as usize;

        let cigar_bytes = n_cigar_op.checked_mul(4).ok_or_else(|| {
            BamRecordViewError::new("BAM record CIGAR byte count overflowed usize.")
        })?;
        let sequence_bytes = sequence_len.div_ceil(2);
        let quality_bytes = sequence_len;

        let read_name_range =
            VARIABLE_START..checked_end(VARIABLE_START, l_read_name, "read name")?;
        let cigar = read_name_range.end..checked_end(read_name_range.end, cigar_bytes, "CIGAR")?;
        let sequence = cigar.end..checked_end(cigar.end, sequence_bytes, "sequence")?;
        let qualities = sequence.end..checked_end(sequence.end, quality_bytes, "qualities")?;

        if qualities.end > expected_len {
            return Err(BamRecordViewError::new(format!(
                "BAM record declared block size {block_size} but needs at least {} bytes including the prefix.",
                qualities.end
            )));
        }

        let aux = qualities.end..expected_len;
        let read_name_bytes = &raw_record[read_name_range.clone()];
        let Some((&0, read_name_without_nul)) = read_name_bytes.split_last() else {
            return Err(BamRecordViewError::new(
                "BAM record read name was not NUL-terminated.",
            ));
        };
        let read_name = str::from_utf8(read_name_without_nul).map_err(|error| {
            BamRecordViewError::new(format!("BAM record read name is not valid UTF-8: {error}"))
        })?;

        Ok(Self {
            raw_record,
            sections: BamRecordSections {
                raw_record: 0..expected_len,
                core: CORE_START..VARIABLE_START,
                read_name: read_name_range,
                cigar,
                sequence,
                qualities,
                aux,
            },
            block_size,
            ref_id,
            pos,
            bin,
            next_ref_id,
            next_pos,
            template_len,
            flags,
            mapping_quality,
            n_cigar_op,
            sequence_len,
            read_name,
        })
    }

    pub fn raw_record(&self) -> &'a [u8] {
        self.raw_record
    }

    pub fn sections(&self) -> &BamRecordSections {
        &self.sections
    }

    pub fn core_range(&self) -> Range<usize> {
        self.sections.core.clone()
    }

    pub fn read_name_range(&self) -> Range<usize> {
        self.sections.read_name.clone()
    }

    pub fn cigar_range(&self) -> Range<usize> {
        self.sections.cigar.clone()
    }

    pub fn sequence_range(&self) -> Range<usize> {
        self.sections.sequence.clone()
    }

    pub fn quality_range(&self) -> Range<usize> {
        self.sections.qualities.clone()
    }

    pub fn aux_range(&self) -> Range<usize> {
        self.sections.aux.clone()
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn ref_id(&self) -> i32 {
        self.ref_id
    }

    pub fn pos(&self) -> i32 {
        self.pos
    }

    pub fn bin(&self) -> u16 {
        self.bin
    }

    pub fn next_ref_id(&self) -> i32 {
        self.next_ref_id
    }

    pub fn next_pos(&self) -> i32 {
        self.next_pos
    }

    pub fn template_len(&self) -> i32 {
        self.template_len
    }

    pub fn flags(&self) -> u16 {
        self.flags
    }

    pub fn flag_summary(&self) -> BamRecordFlags {
        BamRecordFlags::from_raw(self.flags)
    }

    pub fn coordinates(&self) -> BamRecordCoordinates {
        BamRecordCoordinates {
            ref_id: self.ref_id,
            pos: self.pos,
            next_ref_id: self.next_ref_id,
            next_pos: self.next_pos,
            template_len: self.template_len,
            bin: self.bin,
        }
    }

    /// Return the mapped reference interval without exposing record payload.
    ///
    /// Reference-consuming CIGAR operations are `M`, `D`, `N`, `=`, and `X`.
    /// A mapped record with no reference-consuming operation occupies one base,
    /// matching BAM indexing and region-overlap behavior.
    pub fn mapped_reference_interval(
        &self,
    ) -> Result<Option<BamReferenceInterval>, BamRecordViewError> {
        if self.flag_summary().is_unmapped || self.ref_id < 0 || self.pos < 0 {
            return Ok(None);
        }
        let reference_index = usize::try_from(self.ref_id).map_err(|_| {
            BamRecordViewError::new(
                "Mapped BAM record reference id could not be represented as an index.",
            )
        })?;
        let start = self.pos as u32;
        let mut span = 0_u32;
        for chunk in self.cigar_bytes().chunks_exact(4) {
            let raw = u32::from_le_bytes(chunk.try_into().expect("chunk size checked"));
            let length = raw >> 4;
            let operation = raw & 0x0f;
            if operation > 8 {
                return Err(BamRecordViewError::new(
                    "Mapped BAM record CIGAR contained an unsupported operation code.",
                ));
            }
            if matches!(operation, 0 | 2 | 3 | 7 | 8) {
                span = span.checked_add(length).ok_or_else(|| {
                    BamRecordViewError::new("Mapped BAM record reference span overflowed u32.")
                })?;
            }
        }
        let end = start.checked_add(span.max(1)).ok_or_else(|| {
            BamRecordViewError::new("Mapped BAM record reference interval overflowed u32.")
        })?;
        Ok(Some(BamReferenceInterval {
            reference_index,
            start,
            end,
        }))
    }

    pub fn mapping_quality(&self) -> u8 {
        self.mapping_quality
    }

    pub fn n_cigar_op(&self) -> usize {
        self.n_cigar_op
    }

    pub fn sequence_len(&self) -> usize {
        self.sequence_len
    }

    pub fn read_name(&self) -> &'a str {
        self.read_name
    }

    pub fn skip_offsets(&self) -> BamRecordSkipOffsets {
        BamRecordSkipOffsets {
            after_core: self.sections.core.end,
            after_read_name: self.sections.read_name.end,
            after_cigar: self.sections.cigar.end,
            after_sequence: self.sections.sequence.end,
            after_qualities: self.sections.qualities.end,
            after_aux: self.sections.aux.end,
        }
    }

    pub fn has_cigar(&self) -> bool {
        !self.sections.cigar.is_empty()
    }

    pub fn has_sequence(&self) -> bool {
        self.sequence_len > 0
    }

    pub fn has_qualities(&self) -> bool {
        !self.sections.qualities.is_empty()
    }

    pub fn has_aux(&self) -> bool {
        !self.sections.aux.is_empty()
    }

    pub fn read_name_bytes(&self) -> &'a [u8] {
        &self.raw_record[self.sections.read_name.clone()]
    }

    pub fn cigar_bytes(&self) -> &'a [u8] {
        &self.raw_record[self.sections.cigar.clone()]
    }

    pub fn sequence_bytes(&self) -> &'a [u8] {
        &self.raw_record[self.sections.sequence.clone()]
    }

    pub fn quality_bytes(&self) -> &'a [u8] {
        &self.raw_record[self.sections.qualities.clone()]
    }

    pub fn aux_bytes(&self) -> &'a [u8] {
        &self.raw_record[self.sections.aux.clone()]
    }

    pub fn to_record_layout(&self) -> RecordLayout {
        RecordLayout {
            block_size: self.block_size,
            ref_id: self.ref_id,
            pos: self.pos,
            bin: self.bin,
            next_ref_id: self.next_ref_id,
            next_pos: self.next_pos,
            tlen: self.template_len,
            flags: self.flags,
            mapping_quality: self.mapping_quality,
            n_cigar_op: self.n_cigar_op,
            l_seq: self.sequence_len,
            read_name: self.read_name.to_string(),
            cigar_bytes: self.cigar_bytes().to_vec(),
            sequence_bytes: self.sequence_bytes().to_vec(),
            quality_bytes: self.quality_bytes().to_vec(),
            aux_bytes: self.aux_bytes().to_vec(),
        }
    }
}

impl BamRecordFlags {
    pub fn from_raw(raw: u16) -> Self {
        Self {
            raw,
            is_paired: raw & 0x1 != 0,
            is_proper_pair: raw & 0x2 != 0,
            is_unmapped: raw & 0x4 != 0,
            is_mate_unmapped: raw & 0x8 != 0,
            is_reverse: raw & 0x10 != 0,
            is_mate_reverse: raw & 0x20 != 0,
            is_read1: raw & 0x40 != 0,
            is_read2: raw & 0x80 != 0,
            is_secondary: raw & 0x100 != 0,
            is_qc_fail: raw & 0x200 != 0,
            is_duplicate: raw & 0x400 != 0,
            is_supplementary: raw & 0x800 != 0,
        }
    }

    pub fn is_primary(self) -> bool {
        !self.is_secondary && !self.is_supplementary
    }
}

fn checked_end(start: usize, len: usize, label: &'static str) -> Result<usize, BamRecordViewError> {
    start.checked_add(len).ok_or_else(|| {
        BamRecordViewError::new(format!("BAM record {label} range overflowed usize."))
    })
}

fn read_i32_le(raw_record: &[u8], offset: usize) -> Result<i32, BamRecordViewError> {
    let bytes = read_4(raw_record, offset)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u32_le(raw_record: &[u8], offset: usize) -> Result<u32, BamRecordViewError> {
    let bytes = read_4(raw_record, offset)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_4(raw_record: &[u8], offset: usize) -> Result<[u8; 4], BamRecordViewError> {
    let end = offset.checked_add(4).ok_or_else(|| {
        BamRecordViewError::new("BAM record fixed-field offset overflowed usize.")
    })?;
    let bytes = raw_record.get(offset..end).ok_or_else(|| {
        BamRecordViewError::new("BAM record ended before the core fields were available.")
    })?;
    Ok(bytes.try_into().expect("slice length checked above"))
}

#[cfg(test)]
mod tests {
    use crate::{
        bam::{record::BamRecordView, write::serialize_record_layout},
        bgzf::test_support::build_light_record,
    };

    #[test]
    fn parses_minimal_light_record_without_materializing_sections() {
        let raw = build_light_record(0, 7, "read1", 0x10);
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        assert_eq!(view.block_size(), raw.len() - 4);
        assert_eq!(view.ref_id(), 0);
        assert_eq!(view.pos(), 7);
        assert_eq!(view.flags(), 0x10);
        assert_eq!(view.mapping_quality(), 0);
        assert_eq!(view.read_name(), "read1");
        assert_eq!(view.sequence_len(), 0);
        assert_eq!(view.sections().core, 4..36);
        assert_eq!(view.sections().read_name, 36..42);
        assert_eq!(view.sections().cigar, 42..42);
        assert_eq!(view.sections().sequence, 42..42);
        assert_eq!(view.sections().qualities, 42..42);
        assert_eq!(view.sections().aux, 42..42);
        assert!(view.cigar_bytes().is_empty());
        assert!(view.sequence_bytes().is_empty());
        assert!(view.quality_bytes().is_empty());
        assert!(view.aux_bytes().is_empty());
    }

    #[test]
    fn exposes_variable_section_boundaries() {
        let raw = build_record_with_sections();
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        assert_eq!(view.ref_id(), 2);
        assert_eq!(view.pos(), 100);
        assert_eq!(view.mapping_quality(), 42);
        assert_eq!(view.flags(), 0x41);
        assert_eq!(view.read_name(), "readA");
        assert_eq!(view.sequence_len(), 5);
        assert_eq!(view.n_cigar_op(), 1);
        assert_eq!(view.next_ref_id(), 2);
        assert_eq!(view.next_pos(), 150);
        assert_eq!(view.template_len(), 200);
        assert_eq!(view.sections().read_name, 36..42);
        assert_eq!(view.sections().cigar, 42..46);
        assert_eq!(view.sections().sequence, 46..49);
        assert_eq!(view.sections().qualities, 49..54);
        assert_eq!(view.sections().aux, 54..61);
        assert_eq!(view.read_name_range(), 36..42);
        assert_eq!(view.cigar_range(), 42..46);
        assert_eq!(view.sequence_range(), 46..49);
        assert_eq!(view.quality_range(), 49..54);
        assert_eq!(view.aux_range(), 54..61);
        assert_eq!(view.cigar_bytes(), &[160, 0, 0, 0]);
        assert_eq!(view.sequence_bytes(), &[0x12, 0x48, 0xf0]);
        assert_eq!(view.quality_bytes(), &[30, 31, 32, 33, 34]);
        assert_eq!(view.aux_bytes(), b"NMi\x01\0\0\0");
        assert!(view.has_cigar());
        assert!(view.has_sequence());
        assert!(view.has_qualities());
        assert!(view.has_aux());
        assert_eq!(
            view.skip_offsets(),
            super::BamRecordSkipOffsets {
                after_core: 36,
                after_read_name: 42,
                after_cigar: 46,
                after_sequence: 49,
                after_qualities: 54,
                after_aux: 61,
            }
        );
    }

    #[test]
    fn exposes_central_flag_and_coordinate_helpers() {
        let raw = build_record_with_sections();
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let flags = view.flag_summary();
        assert_eq!(flags.raw, 0x41);
        assert!(flags.is_paired);
        assert!(!flags.is_proper_pair);
        assert!(!flags.is_unmapped);
        assert!(flags.is_read1);
        assert!(!flags.is_read2);
        assert!(flags.is_primary());
        assert!(!flags.is_duplicate);

        assert_eq!(
            view.coordinates(),
            super::BamRecordCoordinates {
                ref_id: 2,
                pos: 100,
                next_ref_id: 2,
                next_pos: 150,
                template_len: 200,
                bin: 4681,
            }
        );
        assert_eq!(view.mapping_quality(), 42);
        assert_eq!(view.sequence_len(), 5);
        assert_eq!(view.read_name(), "readA");
        assert_eq!(
            view.mapped_reference_interval().unwrap(),
            Some(super::BamReferenceInterval {
                reference_index: 2,
                start: 100,
                end: 110,
            })
        );
    }

    #[test]
    fn mapped_interval_counts_only_reference_consuming_cigar_operations() {
        let raw = build_mapped_record_with_cigar(&[
            (2, 4),
            (3, 0),
            (1, 1),
            (2, 7),
            (4, 2),
            (5, 3),
            (1, 5),
        ]);
        let view = BamRecordView::parse(&raw).unwrap();
        assert_eq!(
            view.mapped_reference_interval().unwrap(),
            Some(super::BamReferenceInterval {
                reference_index: 2,
                start: 100,
                end: 114,
            })
        );
    }

    #[test]
    fn exposes_unmapped_flags_and_empty_skip_sections() {
        let raw = build_light_record(-1, -1, "unmapped", 0x4 | 0x100 | 0x400);
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let flags = view.flag_summary();
        assert!(flags.is_unmapped);
        assert!(flags.is_secondary);
        assert!(flags.is_duplicate);
        assert!(!flags.is_primary());
        assert_eq!(view.coordinates().ref_id, -1);
        assert_eq!(view.coordinates().pos, -1);
        assert_eq!(view.sequence_len(), 0);
        assert!(!view.has_cigar());
        assert!(!view.has_sequence());
        assert!(!view.has_qualities());
        assert!(!view.has_aux());
        assert_eq!(view.mapped_reference_interval().unwrap(), None);

        let offsets = view.skip_offsets();
        assert_eq!(offsets.after_core, 36);
        assert_eq!(offsets.after_read_name, view.raw_record().len());
        assert_eq!(offsets.after_cigar, view.raw_record().len());
        assert_eq!(offsets.after_sequence, view.raw_record().len());
        assert_eq!(offsets.after_qualities, view.raw_record().len());
        assert_eq!(offsets.after_aux, view.raw_record().len());
    }

    #[test]
    fn bridge_to_record_layout_preserves_lossless_serialization() {
        let raw = build_record_with_sections();
        let view = BamRecordView::parse(&raw).expect("record view should parse");
        let layout = view.to_record_layout();

        assert_eq!(serialize_record_layout(&layout), raw);
        assert_eq!(layout.read_name, "readA");
        assert_eq!(layout.sequence_bytes, vec![0x12, 0x48, 0xf0]);
        assert_eq!(layout.aux_bytes, b"NMi\x01\0\0\0");
    }

    #[test]
    fn rejects_variable_sections_that_exceed_declared_block_size() {
        let mut raw = build_light_record(0, 7, "read1", 0);
        raw[20..24].copy_from_slice(&10_i32.to_le_bytes());

        let error = BamRecordView::parse(&raw).expect_err("record should be rejected");
        assert!(error.detail().contains("needs at least"));
    }

    #[test]
    fn rejects_non_nul_terminated_read_names() {
        let mut raw = build_light_record(0, 7, "read1", 0);
        let final_read_name_byte = 41;
        raw[final_read_name_byte] = b'!';

        let error = BamRecordView::parse(&raw).expect_err("record should be rejected");
        assert!(error.detail().contains("not NUL-terminated"));
    }

    fn build_record_with_sections() -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(b"readA\0");
        variable.extend_from_slice(&((10_u32) << 4).to_le_bytes());
        variable.extend_from_slice(&[0x12, 0x48, 0xf0]);
        variable.extend_from_slice(&[30, 31, 32, 33, 34]);
        variable.extend_from_slice(b"NMi\x01\0\0\0");

        let block_size = 32 + variable.len();
        let bin_mq_nl = (4681_u32 << 16) | (42_u32 << 8) | 6_u32;
        let flag_nc = (0x41_u32 << 16) | 1_u32;

        let mut raw = Vec::with_capacity(4 + block_size);
        raw.extend_from_slice(&(block_size as i32).to_le_bytes());
        raw.extend_from_slice(&2_i32.to_le_bytes());
        raw.extend_from_slice(&100_i32.to_le_bytes());
        raw.extend_from_slice(&bin_mq_nl.to_le_bytes());
        raw.extend_from_slice(&flag_nc.to_le_bytes());
        raw.extend_from_slice(&5_i32.to_le_bytes());
        raw.extend_from_slice(&2_i32.to_le_bytes());
        raw.extend_from_slice(&150_i32.to_le_bytes());
        raw.extend_from_slice(&200_i32.to_le_bytes());
        raw.extend_from_slice(&variable);
        raw
    }

    fn build_mapped_record_with_cigar(operations: &[(u32, u32)]) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(b"readA\0");
        for (length, operation) in operations {
            variable.extend_from_slice(&((length << 4) | operation).to_le_bytes());
        }
        let block_size = 32 + variable.len();
        let bin_mq_nl = (4681_u32 << 16) | (42_u32 << 8) | 6_u32;
        let flag_nc = (0x41_u32 << 16) | operations.len() as u32;
        let mut raw = Vec::with_capacity(4 + block_size);
        raw.extend_from_slice(&(block_size as i32).to_le_bytes());
        raw.extend_from_slice(&2_i32.to_le_bytes());
        raw.extend_from_slice(&100_i32.to_le_bytes());
        raw.extend_from_slice(&bin_mq_nl.to_le_bytes());
        raw.extend_from_slice(&flag_nc.to_le_bytes());
        raw.extend_from_slice(&0_i32.to_le_bytes());
        raw.extend_from_slice(&2_i32.to_le_bytes());
        raw.extend_from_slice(&150_i32.to_le_bytes());
        raw.extend_from_slice(&200_i32.to_le_bytes());
        raw.extend_from_slice(&variable);
        raw
    }
}
