use crate::{
    bam::{reader::BamReader, records::read_next_record_layout},
    error::AppError,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuxField<'a> {
    pub tag: [u8; 2],
    pub type_code: u8,
    pub payload: &'a [u8],
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
    let Some(layout) = read_next_record_layout(reader)? else {
        return Ok(None);
    };
    let matched = aux_region_contains_tag(&layout.aux_bytes, query).map_err(|detail| {
        AppError::InvalidRecord {
            path: reader.path().to_path_buf(),
            detail,
        }
    })?;

    Ok(Some(TagScanRecordResult { matched }))
}

pub fn aux_region_contains_tag(aux_bytes: &[u8], query: TagQuery) -> Result<bool, String> {
    let mut matched = false;
    traverse_aux_fields(aux_bytes, |field| {
        let matches_type = query
            .required_type
            .is_none_or(|required| aux_type_matches(field.type_code, required));
        if field.tag == query.tag && matches_type {
            matched = true;
        }
        Ok(())
    })?;

    Ok(matched)
}

pub fn extract_string_aux_tag(
    aux_bytes: &[u8],
    query_tag: [u8; 2],
) -> Result<Option<String>, String> {
    let mut value = None;
    traverse_aux_fields(aux_bytes, |field| {
        if field.tag == query_tag {
            if field.type_code != b'Z' {
                return Err(format!(
                    "Auxiliary tag {}{} was present but not a Z string.",
                    query_tag[0] as char, query_tag[1] as char
                ));
            }

            let string_bytes = field.payload.strip_suffix(&[0]).ok_or_else(|| {
                "Encountered a malformed NUL-terminated auxiliary string.".to_string()
            })?;
            let parsed = String::from_utf8(string_bytes.to_vec()).map_err(|error| {
                format!("BAM auxiliary string tag was not valid UTF-8: {error}")
            })?;
            value = Some(parsed);
        }
        Ok(())
    })?;

    Ok(value)
}

pub fn traverse_aux_fields(
    aux_bytes: &[u8],
    mut visitor: impl FnMut(AuxField<'_>) -> Result<(), String>,
) -> Result<(), String> {
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
        let payload_end = offset.checked_add(payload_len).ok_or_else(|| {
            "Auxiliary field payload length overflowed usize during traversal.".to_string()
        })?;
        let payload = &aux_bytes[offset..payload_end];

        visitor(AuxField {
            tag,
            type_code,
            payload,
        })?;

        offset = payload_end;
    }

    Ok(())
}

pub fn serialize_filtered_aux(
    aux_bytes: &[u8],
    excluded_tags: &std::collections::HashSet<[u8; 2]>,
) -> Result<Vec<u8>, String> {
    let mut serialized = Vec::new();
    traverse_aux_fields(aux_bytes, |field| {
        if !excluded_tags.contains(&field.tag) {
            serialized.extend_from_slice(&field.tag);
            serialized.push(field.type_code);
            serialized.extend_from_slice(field.payload);
        }
        Ok(())
    })?;
    Ok(serialized)
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
    use super::{
        AuxTypeCode, TagQuery, aux_region_contains_tag, extract_string_aux_tag, validate_tag,
    };

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

    #[test]
    fn extracts_string_aux_tag_values() {
        let aux = b"RGZgroup1\0NMi\x01\0\0\0";
        let value = extract_string_aux_tag(aux, *b"RG").expect("tag extraction should succeed");
        assert_eq!(value.as_deref(), Some("group1"));
    }
}
