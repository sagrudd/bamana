use crate::{bam::reader::BamReader, error::AppError};

const BAM_CORE_SIZE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxTypeCode {
    A,
    CLower,
    CUpper,
    SLower,
    SUpper,
    ILower,
    IUpper,
    F,
    Z,
    H,
    B,
}

impl AuxTypeCode {
    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "A" => Some(Self::A),
            "c" => Some(Self::CLower),
            "C" => Some(Self::CUpper),
            "s" => Some(Self::SLower),
            "S" => Some(Self::SUpper),
            "i" => Some(Self::ILower),
            "I" => Some(Self::IUpper),
            "f" => Some(Self::F),
            "Z" => Some(Self::Z),
            "H" => Some(Self::H),
            "B" => Some(Self::B),
            _ => None,
        }
    }

    pub fn as_char(self) -> char {
        match self {
            Self::A => 'A',
            Self::CLower => 'c',
            Self::CUpper => 'C',
            Self::SLower => 's',
            Self::SUpper => 'S',
            Self::ILower => 'i',
            Self::IUpper => 'I',
            Self::F => 'f',
            Self::Z => 'Z',
            Self::H => 'H',
            Self::B => 'B',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagQuery {
    pub tag: [u8; 2],
    pub required_type: Option<AuxTypeCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagScanRecordResult {
    pub matched: bool,
}

pub fn validate_tag(tag: &str) -> Option<[u8; 2]> {
    let bytes = tag.as_bytes();
    if bytes.len() != 2 || !bytes.iter().all(|byte| byte.is_ascii_graphic()) {
        return None;
    }

    Some([bytes[0], bytes[1]])
}

pub fn read_next_record_for_tag(
    reader: &mut BamReader,
    query: TagQuery,
) -> Result<Option<TagScanRecordResult>, AppError> {
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
    let _ref_id = reader.read_i32_le()?;
    let _pos = reader.read_i32_le()?;
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
    let n_cigar_op = (flag_nc & 0xffff) as usize;

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
    if read_name_bytes.last().copied() != Some(0) {
        return Err(AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail: "BAM record read name was not NUL-terminated.".to_string(),
        });
    }

    reader.skip_exact(cigar_bytes)?;
    reader.skip_exact(sequence_bytes)?;
    reader.skip_exact(quality_bytes)?;

    let aux_len = remaining - consumed_after_core;
    let aux_bytes = reader.read_exact_vec(aux_len)?;
    let matched =
        aux_region_contains_tag(&aux_bytes, query).map_err(|detail| AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail,
        })?;

    Ok(Some(TagScanRecordResult { matched }))
}

pub fn aux_region_contains_tag(aux_bytes: &[u8], query: TagQuery) -> Result<bool, String> {
    let mut offset = 0_usize;

    while offset < aux_bytes.len() {
        if aux_bytes.len() - offset < 3 {
            return Err(
                "Encountered a truncated auxiliary field before a stable conclusion was reached."
                    .to_string(),
            );
        }

        let tag = [aux_bytes[offset], aux_bytes[offset + 1]];
        let type_code = aux_bytes[offset + 2];
        offset += 3;

        let payload_len = payload_len(aux_bytes, offset, type_code)?;
        let matches_type = query
            .required_type
            .is_none_or(|required| aux_type_matches(type_code, required));

        if tag == query.tag && matches_type {
            return Ok(true);
        }

        offset = offset.checked_add(payload_len).ok_or_else(|| {
            "Auxiliary field payload length overflowed usize during traversal.".to_string()
        })?;
    }

    Ok(false)
}

fn payload_len(aux_bytes: &[u8], offset: usize, type_code: u8) -> Result<usize, String> {
    match type_code {
        b'A' | b'c' | b'C' => require_remaining(aux_bytes, offset, 1),
        b's' | b'S' => require_remaining(aux_bytes, offset, 2),
        b'i' | b'I' | b'f' => require_remaining(aux_bytes, offset, 4),
        b'Z' | b'H' => {
            let Some(end) = aux_bytes[offset..].iter().position(|byte| *byte == 0) else {
                return Err("Encountered a malformed NUL-terminated auxiliary string.".to_string());
            };
            Ok(end + 1)
        }
        b'B' => b_array_payload_len(aux_bytes, offset),
        _ => Err(format!(
            "Encountered unsupported or malformed BAM auxiliary type code '{}'.",
            type_code as char
        )),
    }
}

fn b_array_payload_len(aux_bytes: &[u8], offset: usize) -> Result<usize, String> {
    require_remaining(aux_bytes, offset, 5)?;

    let subtype = aux_bytes[offset];
    let element_size = match subtype {
        b'c' | b'C' => 1_usize,
        b's' | b'S' => 2_usize,
        b'i' | b'I' | b'f' => 4_usize,
        _ => {
            return Err(format!(
                "Encountered unsupported BAM auxiliary B-array subtype '{}'.",
                subtype as char
            ));
        }
    };

    let mut count_bytes = [0_u8; 4];
    count_bytes.copy_from_slice(&aux_bytes[offset + 1..offset + 5]);
    let count = i32::from_le_bytes(count_bytes);
    if count < 0 {
        return Err(
            "Encountered a BAM auxiliary B-array with a negative element count.".to_string(),
        );
    }
    let count = count as usize;

    let payload_bytes = count
        .checked_mul(element_size)
        .ok_or_else(|| "BAM auxiliary B-array payload length overflowed usize.".to_string())?;
    require_remaining(aux_bytes, offset + 5, payload_bytes)?;

    Ok(1 + 4 + payload_bytes)
}

fn require_remaining(aux_bytes: &[u8], offset: usize, len: usize) -> Result<usize, String> {
    if aux_bytes.len().saturating_sub(offset) < len {
        return Err(
            "Encountered a truncated auxiliary field before a stable conclusion was reached."
                .to_string(),
        );
    }

    Ok(len)
}

fn aux_type_matches(type_code: u8, required: AuxTypeCode) -> bool {
    type_code == required.as_char() as u8
}

#[cfg(test)]
mod tests {
    use super::{AuxTypeCode, TagQuery, aux_region_contains_tag, validate_tag};

    #[test]
    fn validates_two_character_ascii_tags() {
        assert_eq!(validate_tag("NM"), Some(*b"NM"));
        assert_eq!(validate_tag("N"), None);
        assert_eq!(validate_tag("NM3"), None);
        assert_eq!(validate_tag("N "), None);
    }

    #[test]
    fn finds_string_tag() {
        let aux = b"RGZgroup1\0NMi\x01\0\0\0";
        let matched = aux_region_contains_tag(
            aux,
            TagQuery {
                tag: *b"RG",
                required_type: Some(AuxTypeCode::Z),
            },
        )
        .expect("aux scan should succeed");
        assert!(matched);
    }

    #[test]
    fn respects_type_constraint() {
        let aux = b"RGZgroup1\0";
        let matched = aux_region_contains_tag(
            aux,
            TagQuery {
                tag: *b"RG",
                required_type: Some(AuxTypeCode::H),
            },
        )
        .expect("aux scan should succeed");
        assert!(!matched);
    }

    #[test]
    fn traverses_b_array_payloads() {
        let aux = b"MLBc\x03\0\0\0\x01\x02\x03NMi\x01\0\0\0";
        let matched = aux_region_contains_tag(
            aux,
            TagQuery {
                tag: *b"NM",
                required_type: Some(AuxTypeCode::ILower),
            },
        )
        .expect("aux scan should succeed");
        assert!(matched);
    }

    #[test]
    fn rejects_unterminated_strings() {
        let aux = b"RGZgroup1";
        let error = aux_region_contains_tag(
            aux,
            TagQuery {
                tag: *b"RG",
                required_type: None,
            },
        )
        .expect_err("unterminated string should fail");
        assert!(error.contains("malformed NUL-terminated auxiliary string"));
    }

    #[test]
    fn rejects_truncated_b_arrays() {
        let aux = b"MLBc\x03\0\0\0\x01";
        let error = aux_region_contains_tag(
            aux,
            TagQuery {
                tag: *b"ML",
                required_type: Some(AuxTypeCode::B),
            },
        )
        .expect_err("truncated b-array should fail");
        assert!(error.contains("truncated auxiliary field"));
    }
}
