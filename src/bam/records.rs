use crate::{bam::reader::BamReader, error::AppError};

const BAM_CORE_SIZE: usize = 32;
pub const BAM_FUNMAP: u16 = 0x4;
const BAM_FPAIRED: u16 = 0x1;
const BAM_FPROPER_PAIR: u16 = 0x2;
const BAM_FREVERSE: u16 = 0x10;
const BAM_FREAD1: u16 = 0x40;
const BAM_FREAD2: u16 = 0x80;
const BAM_FSECONDARY: u16 = 0x100;
const BAM_FQCFAIL: u16 = 0x200;
const BAM_FDUP: u16 = 0x400;
const BAM_FSUPPLEMENTARY: u16 = 0x800;

#[derive(Clone, Debug)]
pub struct LightAlignmentRecord {
    pub ref_id: i32,
    pub pos: i32,
    pub flags: u16,
    pub mapping_quality: u8,
    pub read_name: String,
    pub is_unmapped: bool,
    pub is_paired: bool,
    pub is_proper_pair: bool,
    pub is_reverse: bool,
    pub is_secondary: bool,
    pub is_supplementary: bool,
    pub is_qc_fail: bool,
    pub is_duplicate: bool,
    pub is_read1: bool,
    pub is_read2: bool,
}

#[derive(Clone, Debug)]
pub struct RecordLayout {
    pub block_size: usize,
    pub ref_id: i32,
    pub pos: i32,
    pub bin: u16,
    pub next_ref_id: i32,
    pub next_pos: i32,
    pub tlen: i32,
    pub flags: u16,
    pub mapping_quality: u8,
    pub n_cigar_op: usize,
    pub l_seq: usize,
    pub read_name: String,
    pub cigar_bytes: Vec<u8>,
    pub sequence_bytes: Vec<u8>,
    pub quality_bytes: Vec<u8>,
    pub aux_bytes: Vec<u8>,
}

pub fn reg2bin(start: i32, end: i32) -> u16 {
    if start < 0 || end <= start {
        return 4680;
    }

    let start = start as u32;
    let end = (end - 1) as u32;

    if start >> 14 == end >> 14 {
        return ((1 << 15) - 1) / 7 + (start >> 14) as u16;
    }
    if start >> 17 == end >> 17 {
        return ((1 << 12) - 1) / 7 + (start >> 17) as u16;
    }
    if start >> 20 == end >> 20 {
        return ((1 << 9) - 1) / 7 + (start >> 20) as u16;
    }
    if start >> 23 == end >> 23 {
        return ((1 << 6) - 1) / 7 + (start >> 23) as u16;
    }
    if start >> 26 == end >> 26 {
        return 1 + (start >> 26) as u16;
    }
    0
}

pub fn encode_bam_sequence(sequence: &str) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::with_capacity(sequence.len().div_ceil(2));
    let bytes = sequence.as_bytes();

    for chunk in bytes.chunks(2) {
        let high = encode_base(chunk[0])?;
        let low = if chunk.len() == 2 {
            encode_base(chunk[1])?
        } else {
            0
        };
        encoded.push((high << 4) | low);
    }

    Ok(encoded)
}

pub fn encode_bam_qualities(qualities: &str) -> Result<Vec<u8>, String> {
    qualities
        .bytes()
        .map(|byte| {
            if byte < 33 {
                Err(format!(
                    "FASTQ/SAM quality byte 0x{byte:02x} is below printable Phred+33 range."
                ))
            } else {
                Ok(byte - 33)
            }
        })
        .collect()
}

pub fn missing_quality_scores(length: usize) -> Vec<u8> {
    vec![0xff; length]
}

pub fn read_next_light_record(
    reader: &mut BamReader,
) -> Result<Option<LightAlignmentRecord>, AppError> {
    read_next_record_layout(reader).map(|layout| layout.map(light_from_layout))
}

pub fn read_next_record_layout(reader: &mut BamReader) -> Result<Option<RecordLayout>, AppError> {
    let Some(block_size) = reader.read_optional_i32_le()? else {
        return Ok(None);
    };

    if block_size < BAM_CORE_SIZE as i32 {
        return Err(AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: format!(
                "BAM record block size {block_size} is smaller than the 32-byte core alignment section."
            ),
        });
    }

    let block_size = block_size as usize;
    let ref_id = reader.read_i32_le()?;
    let pos = reader.read_i32_le()?;
    let bin_mq_nl = reader.read_u32_le()?;
    let flag_nc = reader.read_u32_le()?;
    let l_seq = reader.read_i32_le()?;
    let next_ref_id = reader.read_i32_le()?;
    let next_pos = reader.read_i32_le()?;
    let tlen = reader.read_i32_le()?;

    if l_seq < 0 {
        return Err(AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: "BAM record sequence length was negative.".to_string(),
        });
    }

    let remaining = block_size - BAM_CORE_SIZE;
    let l_read_name = (bin_mq_nl & 0xff) as usize;
    let mapping_quality = ((bin_mq_nl >> 8) & 0xff) as u8;
    let bin = (bin_mq_nl >> 16) as u16;
    let n_cigar_op = (flag_nc & 0xffff) as usize;
    let flags = (flag_nc >> 16) as u16;

    if l_read_name == 0 {
        return Err(AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: "BAM record read name length was zero.".to_string(),
        });
    }

    let cigar_bytes = n_cigar_op
        .checked_mul(4)
        .ok_or_else(|| AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: "BAM record CIGAR byte count overflowed usize.".to_string(),
        })?;
    let l_seq = l_seq as usize;
    let sequence_bytes = l_seq.div_ceil(2);
    let quality_bytes = l_seq;

    let consumed_after_core = l_read_name
        .checked_add(cigar_bytes)
        .and_then(|value| value.checked_add(sequence_bytes))
        .and_then(|value| value.checked_add(quality_bytes))
        .ok_or_else(|| AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: "BAM record variable-length section overflowed usize.".to_string(),
        })?;

    if consumed_after_core > remaining {
        return Err(AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: format!(
                "BAM record declared block size {block_size} but needs at least {} bytes after the core section.",
                consumed_after_core
            ),
        });
    }

    let read_name_bytes = reader.read_exact_vec(l_read_name)?;
    let Some((&0, read_name_without_nul)) = read_name_bytes.split_last() else {
        return Err(AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: "BAM record read name was not NUL-terminated.".to_string(),
        });
    };

    let read_name = String::from_utf8(read_name_without_nul.to_vec()).map_err(|error| {
        AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: format!("BAM record read name is not valid UTF-8: {error}"),
        }
    })?;

    let cigar_bytes = reader.read_exact_vec(cigar_bytes)?;
    let sequence_bytes = reader.read_exact_vec(sequence_bytes)?;
    let quality_bytes = reader.read_exact_vec(quality_bytes)?;
    let aux_bytes = reader.read_exact_vec(remaining - consumed_after_core)?;

    Ok(Some(RecordLayout {
        block_size,
        ref_id,
        pos,
        bin,
        next_ref_id,
        next_pos,
        tlen,
        flags,
        mapping_quality,
        n_cigar_op,
        l_seq,
        read_name,
        cigar_bytes,
        sequence_bytes,
        quality_bytes,
        aux_bytes,
    }))
}

fn light_from_layout(layout: RecordLayout) -> LightAlignmentRecord {
    let flags = layout.flags;
    LightAlignmentRecord {
        ref_id: layout.ref_id,
        pos: layout.pos,
        flags,
        mapping_quality: layout.mapping_quality,
        read_name: layout.read_name,
        is_unmapped: flags & BAM_FUNMAP != 0,
        is_paired: flags & BAM_FPAIRED != 0,
        is_proper_pair: flags & BAM_FPROPER_PAIR != 0,
        is_reverse: flags & BAM_FREVERSE != 0,
        is_secondary: flags & BAM_FSECONDARY != 0,
        is_supplementary: flags & BAM_FSUPPLEMENTARY != 0,
        is_qc_fail: flags & BAM_FQCFAIL != 0,
        is_duplicate: flags & BAM_FDUP != 0,
        is_read1: flags & BAM_FREAD1 != 0,
        is_read2: flags & BAM_FREAD2 != 0,
    }
}

fn encode_base(base: u8) -> Result<u8, String> {
    match base.to_ascii_uppercase() {
        b'=' => Ok(0),
        b'A' => Ok(1),
        b'C' => Ok(2),
        b'M' => Ok(3),
        b'G' => Ok(4),
        b'R' => Ok(5),
        b'S' => Ok(6),
        b'V' => Ok(7),
        b'T' => Ok(8),
        b'W' => Ok(9),
        b'Y' => Ok(10),
        b'H' => Ok(11),
        b'K' => Ok(12),
        b'D' => Ok(13),
        b'B' => Ok(14),
        b'N' => Ok(15),
        other => Err(format!(
            "Sequence contains unsupported base byte 0x{other:02x}."
        )),
    }
}
