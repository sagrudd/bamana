use crate::{bam::reader::BamReader, error::AppError};

const BAM_CORE_SIZE: usize = 32;
const BAM_FPAIRED: u16 = 0x1;
const BAM_FPROPER_PAIR: u16 = 0x2;
const BAM_FUNMAP: u16 = 0x4;
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

pub fn read_next_light_record(
    reader: &mut BamReader,
) -> Result<Option<LightAlignmentRecord>, AppError> {
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
    let _next_ref_id = reader.read_i32_le()?;
    let _next_pos = reader.read_i32_le()?;
    let _tlen = reader.read_i32_le()?;

    if l_seq < 0 {
        return Err(AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: "BAM record sequence length was negative.".to_string(),
        });
    }

    let remaining = block_size - BAM_CORE_SIZE;
    let l_read_name = (bin_mq_nl & 0xff) as usize;
    let mapping_quality = ((bin_mq_nl >> 8) & 0xff) as u8;
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

    reader.skip_exact(cigar_bytes)?;
    reader.skip_exact(sequence_bytes)?;
    reader.skip_exact(quality_bytes)?;
    reader.skip_exact(remaining - consumed_after_core)?;

    Ok(Some(LightAlignmentRecord {
        ref_id,
        pos,
        flags,
        mapping_quality,
        read_name,
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
    }))
}
