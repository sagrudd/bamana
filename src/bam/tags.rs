use crate::{
    bam::{reader::BamReader, record::BamRecordView, records::read_next_record_layout},
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

pub fn record_aux_contains_tag(
    record: &BamRecordView<'_>,
    query: TagQuery,
) -> Result<bool, String> {
    aux_region_contains_tag(record.aux_bytes(), query)
}

pub fn count_aux_tag(aux_bytes: &[u8], query: TagQuery) -> Result<u64, String> {
    let mut count = 0_u64;
    traverse_aux_fields(aux_bytes, |field| {
        let matches_type = query
            .required_type
            .is_none_or(|required| aux_type_matches(field.type_code, required));
        if field.tag == query.tag && matches_type {
            count += 1;
        }
        Ok(())
    })?;

    Ok(count)
}

pub fn count_record_aux_tag(record: &BamRecordView<'_>, query: TagQuery) -> Result<u64, String> {
    count_aux_tag(record.aux_bytes(), query)
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

pub fn extract_record_string_aux_tag(
    record: &BamRecordView<'_>,
    query_tag: [u8; 2],
) -> Result<Option<String>, String> {
    extract_string_aux_tag(record.aux_bytes(), query_tag)
}

pub fn collect_aux_tag_keys(aux_bytes: &[u8]) -> Result<Vec<[u8; 2]>, String> {
    let mut tags = Vec::new();
    traverse_aux_fields(aux_bytes, |field| {
        tags.push(field.tag);
        Ok(())
    })?;
    Ok(tags)
}

pub fn collect_record_aux_tag_keys(record: &BamRecordView<'_>) -> Result<Vec<[u8; 2]>, String> {
    collect_aux_tag_keys(record.aux_bytes())
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

pub fn traverse_record_aux_fields(
    record: &BamRecordView<'_>,
    visitor: impl FnMut(AuxField<'_>) -> Result<(), String>,
) -> Result<(), String> {
    traverse_aux_fields(record.aux_bytes(), visitor)
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
        AuxTypeCode, TagQuery, aux_region_contains_tag, collect_record_aux_tag_keys,
        count_record_aux_tag, extract_record_string_aux_tag, extract_string_aux_tag,
        record_aux_contains_tag, traverse_record_aux_fields, validate_tag,
    };
    use crate::bam::record::BamRecordView;

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

    #[test]
    fn record_view_finds_scalar_tags_without_full_decode() {
        let raw = build_record_with_aux(b"NMc\x05ASi*\0\0\0");
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let matched = record_aux_contains_tag(
            &view,
            TagQuery {
                tag: *b"NM",
                required_type: Some(AuxTypeCode::CLower),
            },
        )
        .expect("record aux scan should succeed");

        assert!(matched);
        assert_eq!(
            collect_record_aux_tag_keys(&view).expect("tag keys should collect"),
            vec![*b"NM", *b"AS"]
        );
    }

    #[test]
    fn record_view_extracts_string_tags_for_read_group_evidence() {
        let raw = build_record_with_aux(b"RGZgroup1\0NMc\x05");
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let value =
            extract_record_string_aux_tag(&view, *b"RG").expect("record string tag should parse");

        assert_eq!(value.as_deref(), Some("group1"));
    }

    #[test]
    fn record_view_skips_arrays_when_finding_later_tags() {
        let raw = build_record_with_aux(b"MLBc\x03\0\0\0\x01\x02\x03NMc\x02");
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let matched = record_aux_contains_tag(
            &view,
            TagQuery {
                tag: *b"NM",
                required_type: Some(AuxTypeCode::CLower),
            },
        )
        .expect("record aux scan should succeed");

        assert!(matched);
    }

    #[test]
    fn record_view_distinguishes_missing_tags_from_malformed_aux() {
        let raw = build_record_with_aux(b"RGZgroup1\0");
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let matched = record_aux_contains_tag(
            &view,
            TagQuery {
                tag: *b"NM",
                required_type: None,
            },
        )
        .expect("well-formed missing tag should not fail");

        assert!(!matched);
    }

    #[test]
    fn record_view_counts_duplicate_tags() {
        let raw = build_record_with_aux(b"NMc\x01NMc\x02RGZgroup1\0");
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let count = count_record_aux_tag(
            &view,
            TagQuery {
                tag: *b"NM",
                required_type: Some(AuxTypeCode::CLower),
            },
        )
        .expect("duplicate tag count should succeed");

        assert_eq!(count, 2);
    }

    #[test]
    fn record_view_rejects_malformed_aux_lengths() {
        let raw = build_record_with_aux(b"NMc");
        let view = BamRecordView::parse(&raw).expect("record view should parse");

        let error = record_aux_contains_tag(
            &view,
            TagQuery {
                tag: *b"NM",
                required_type: None,
            },
        )
        .expect_err("truncated scalar payload should fail");

        assert!(error.contains("truncated auxiliary field"));
    }

    #[test]
    fn record_view_traverses_borrowed_aux_fields() {
        let raw = build_record_with_aux(b"RGZgroup1\0NMc\x02");
        let view = BamRecordView::parse(&raw).expect("record view should parse");
        let mut seen = Vec::new();

        traverse_record_aux_fields(&view, |field| {
            seen.push((field.tag, field.type_code, field.payload.len()));
            Ok(())
        })
        .expect("record aux traversal should succeed");

        assert_eq!(seen, vec![(*b"RG", b'Z', 7), (*b"NM", b'c', 1)]);
    }

    fn build_record_with_aux(aux: &[u8]) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(b"read1\0");
        variable.extend_from_slice(aux);

        let block_size = 32 + variable.len();
        let bin_mq_nl = variable
            .len()
            .checked_sub(aux.len())
            .expect("read name length should be present") as u32;
        let flag_nc = 0_u32;

        let mut raw = Vec::with_capacity(4 + block_size);
        raw.extend_from_slice(&(block_size as i32).to_le_bytes());
        raw.extend_from_slice(&0_i32.to_le_bytes());
        raw.extend_from_slice(&1_i32.to_le_bytes());
        raw.extend_from_slice(&bin_mq_nl.to_le_bytes());
        raw.extend_from_slice(&flag_nc.to_le_bytes());
        raw.extend_from_slice(&0_i32.to_le_bytes());
        raw.extend_from_slice(&(-1_i32).to_le_bytes());
        raw.extend_from_slice(&(-1_i32).to_le_bytes());
        raw.extend_from_slice(&0_i32.to_le_bytes());
        raw.extend_from_slice(&variable);
        raw
    }
}
