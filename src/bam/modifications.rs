//! Strict, Bamana-native decoding of the standard cytosine modification trio.
//!
//! SAM `MM` positions are expressed against the record's `SEQ` field. BAM
//! already stores reverse alignments in that orientation, so callers must not
//! reverse the decoded query positions before CIGAR projection.

use std::{error::Error, fmt};

use crate::bam::{record::BamRecordView, tags::traverse_aux_fields};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CytosineModification {
    pub query_position: usize,
    pub reference_position: Option<i64>,
    pub probability: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CytosineModificationTrio {
    pub sequence_length: usize,
    pub modifications: Vec<CytosineModification>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModificationDecodeError {
    MalformedAux(String),
    PartialTrio,
    DuplicateTag([u8; 2]),
    WrongType {
        tag: [u8; 2],
        expected: &'static str,
        observed: u8,
    },
    UnsupportedMmCode(String),
    MalformedMm(String),
    MnMismatch {
        mn: usize,
        sequence_length: usize,
    },
    MlCardinality {
        positions: usize,
        probabilities: usize,
    },
    InvalidCigar(String),
    UnmappedRecord,
}

impl fmt::Display for ModificationDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedAux(detail) => write!(formatter, "malformed BAM aux data: {detail}"),
            Self::PartialTrio => formatter.write_str("MM, ML, and MN must be present atomically"),
            Self::DuplicateTag(tag) => write!(
                formatter,
                "duplicate {}{} tag in modification trio",
                tag[0] as char, tag[1] as char
            ),
            Self::WrongType {
                tag,
                expected,
                observed,
            } => write!(
                formatter,
                "{}{} has BAM type '{}'; expected {expected}",
                tag[0] as char, tag[1] as char, *observed as char
            ),
            Self::UnsupportedMmCode(code) => {
                write!(formatter, "unsupported MM modification code: {code}")
            }
            Self::MalformedMm(detail) => write!(formatter, "malformed MM value: {detail}"),
            Self::MnMismatch {
                mn,
                sequence_length,
            } => write!(
                formatter,
                "MN value {mn} does not equal sequence length {sequence_length}"
            ),
            Self::MlCardinality {
                positions,
                probabilities,
            } => write!(
                formatter,
                "MM contains {positions} calls but ML contains {probabilities} probabilities"
            ),
            Self::InvalidCigar(detail) => write!(formatter, "invalid BAM CIGAR: {detail}"),
            Self::UnmappedRecord => {
                formatter.write_str("cannot project modifications from an unmapped record")
            }
        }
    }
}

impl Error for ModificationDecodeError {}

/// Decode and project a complete `MM:Z:C+m` / `ML:B:C` / `MN:i` trio.
///
/// Returns `Ok(None)` only when all three tags are absent. The supported MM
/// form is one unstranded standard 5mC group (`C+m`) with optional `.` or `?`
/// mode suffix. Other bases, strands, modification codes, and multiple groups
/// are rejected rather than partially interpreted.
pub fn decode_record_c_m_modifications(
    record: &BamRecordView<'_>,
) -> Result<Option<CytosineModificationTrio>, ModificationDecodeError> {
    if record.flag_summary().is_unmapped || record.ref_id() < 0 || record.pos() < 0 {
        return Err(ModificationDecodeError::UnmappedRecord);
    }
    decode_c_m_modifications(
        record.aux_bytes(),
        record.sequence_bytes(),
        record.sequence_len(),
        record.cigar_bytes(),
        i64::from(record.pos()),
    )
}

/// Aux-byte entry point for consumers that already own BAM record sections.
pub fn decode_c_m_modifications(
    aux: &[u8],
    packed_sequence: &[u8],
    sequence_length: usize,
    cigar: &[u8],
    reference_start: i64,
) -> Result<Option<CytosineModificationTrio>, ModificationDecodeError> {
    let Some((mm, ml, mn)) = extract_trio(aux)? else {
        return Ok(None);
    };
    if mn != sequence_length {
        return Err(ModificationDecodeError::MnMismatch {
            mn,
            sequence_length,
        });
    }
    if packed_sequence.len() != sequence_length.div_ceil(2) {
        return Err(ModificationDecodeError::MalformedMm(
            "packed BAM sequence length does not match MN".to_string(),
        ));
    }

    let query_positions = decode_mm_positions(&mm, packed_sequence, sequence_length)?;
    if query_positions.len() != ml.len() {
        return Err(ModificationDecodeError::MlCardinality {
            positions: query_positions.len(),
            probabilities: ml.len(),
        });
    }
    let references = project_positions(&query_positions, cigar, sequence_length, reference_start)?;
    let modifications = query_positions
        .into_iter()
        .zip(references)
        .zip(ml)
        .map(
            |((query_position, reference_position), probability)| CytosineModification {
                query_position,
                reference_position,
                probability,
            },
        )
        .collect();
    Ok(Some(CytosineModificationTrio {
        sequence_length,
        modifications,
    }))
}

fn extract_trio(aux: &[u8]) -> Result<Option<(String, Vec<u8>, usize)>, ModificationDecodeError> {
    let mut mm = None;
    let mut ml = None;
    let mut mn = None;
    traverse_aux_fields(aux, |field| {
        let slot_is_set = match field.tag {
            [b'M', b'M'] => mm.is_some(),
            [b'M', b'L'] => ml.is_some(),
            [b'M', b'N'] => mn.is_some(),
            _ => false,
        };
        if slot_is_set {
            return Err(format!(
                "duplicate modification tag {}{}",
                field.tag[0] as char, field.tag[1] as char
            ));
        }
        match field.tag {
            [b'M', b'M'] => {
                require_type(field.tag, field.type_code, b'Z', "Z")?;
                let bytes = field
                    .payload
                    .strip_suffix(&[0])
                    .ok_or("MM lacks NUL terminator")?;
                mm = Some(
                    std::str::from_utf8(bytes)
                        .map_err(|_| "MM is not UTF-8")?
                        .to_string(),
                );
            }
            [b'M', b'L'] => {
                require_type(field.tag, field.type_code, b'B', "B:C")?;
                if field.payload.len() < 5 || field.payload[0] != b'C' {
                    return Err("ML is not a B:C array".to_string());
                }
                ml = Some(field.payload[5..].to_vec());
            }
            [b'M', b'N'] => {
                require_type(field.tag, field.type_code, b'i', "i")?;
                let bytes: [u8; 4] = field
                    .payload
                    .try_into()
                    .map_err(|_| "MN integer has invalid width")?;
                let value = i32::from_le_bytes(bytes);
                if value < 0 {
                    return Err("MN is negative".to_string());
                }
                mn = Some(value as usize);
            }
            _ => {}
        }
        Ok(())
    })
    .map_err(|detail| {
        if let Some(tag) = duplicate_tag_from_detail(&detail) {
            ModificationDecodeError::DuplicateTag(tag)
        } else if detail.starts_with("wrong type ") {
            parse_wrong_type(&detail)
        } else {
            ModificationDecodeError::MalformedAux(detail)
        }
    })?;

    match (mm, ml, mn) {
        (None, None, None) => Ok(None),
        (Some(mm), Some(ml), Some(mn)) => Ok(Some((mm, ml, mn))),
        _ => Err(ModificationDecodeError::PartialTrio),
    }
}

fn require_type(
    tag: [u8; 2],
    observed: u8,
    required: u8,
    expected: &'static str,
) -> Result<(), String> {
    if observed == required {
        Ok(())
    } else {
        Err(format!(
            "wrong type {}{} {} {}",
            tag[0], tag[1], observed, expected
        ))
    }
}

fn duplicate_tag_from_detail(detail: &str) -> Option<[u8; 2]> {
    let suffix = detail.strip_prefix("duplicate modification tag ")?;
    let bytes = suffix.as_bytes();
    (bytes.len() == 2).then(|| [bytes[0], bytes[1]])
}

fn parse_wrong_type(detail: &str) -> ModificationDecodeError {
    let fields: Vec<_> = detail.split(' ').collect();
    ModificationDecodeError::WrongType {
        tag: [
            fields.get(2).and_then(|v| v.parse().ok()).unwrap_or(b'?'),
            fields.get(3).and_then(|v| v.parse().ok()).unwrap_or(b'?'),
        ],
        observed: fields.get(4).and_then(|v| v.parse().ok()).unwrap_or(b'?'),
        expected: match fields.get(5).copied() {
            Some("Z") => "Z",
            Some("B:C") => "B:C",
            _ => "i",
        },
    }
}

fn decode_mm_positions(
    mm: &str,
    packed_sequence: &[u8],
    sequence_length: usize,
) -> Result<Vec<usize>, ModificationDecodeError> {
    let value = mm
        .strip_suffix(';')
        .ok_or_else(|| ModificationDecodeError::MalformedMm("missing final ';'".to_string()))?;
    if value.contains(';') {
        return Err(ModificationDecodeError::UnsupportedMmCode(
            "multiple MM groups".to_string(),
        ));
    }
    let (code, deltas) = value.split_once(',').unwrap_or((value, ""));
    if !matches!(code, "C+m" | "C+m." | "C+m?") {
        return Err(ModificationDecodeError::UnsupportedMmCode(code.to_string()));
    }
    if deltas.is_empty() {
        return Ok(Vec::new());
    }

    let canonical_positions: Vec<_> = (0..sequence_length)
        .filter(|position| bam_base(packed_sequence, *position) == 2)
        .collect();
    let mut canonical_index = 0_usize;
    let mut positions = Vec::new();
    for delta in deltas.split(',') {
        if delta.is_empty() || !delta.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ModificationDecodeError::MalformedMm(format!(
                "invalid delta '{delta}'"
            )));
        }
        let skipped = delta.parse::<usize>().map_err(|_| {
            ModificationDecodeError::MalformedMm(format!("delta '{delta}' overflows usize"))
        })?;
        canonical_index = canonical_index.checked_add(skipped).ok_or_else(|| {
            ModificationDecodeError::MalformedMm("canonical-base index overflow".to_string())
        })?;
        let position = canonical_positions.get(canonical_index).ok_or_else(|| {
            ModificationDecodeError::MalformedMm(
                "delta addresses beyond canonical cytosines in SEQ".to_string(),
            )
        })?;
        positions.push(*position);
        canonical_index += 1;
    }
    Ok(positions)
}

fn bam_base(sequence: &[u8], position: usize) -> u8 {
    let packed = sequence[position / 2];
    if position % 2 == 0 {
        packed >> 4
    } else {
        packed & 0x0f
    }
}

fn project_positions(
    positions: &[usize],
    cigar: &[u8],
    sequence_length: usize,
    reference_start: i64,
) -> Result<Vec<Option<i64>>, ModificationDecodeError> {
    if cigar.len() % 4 != 0 {
        return Err(ModificationDecodeError::InvalidCigar(
            "byte length is not divisible by four".to_string(),
        ));
    }
    let mut projected = vec![None; positions.len()];
    let mut target = 0_usize;
    let mut query = 0_usize;
    let mut reference = reference_start;
    for encoded in cigar.chunks_exact(4) {
        let value = u32::from_le_bytes(encoded.try_into().expect("exact CIGAR word"));
        let length = (value >> 4) as usize;
        let op = (value & 0xf) as u8;
        if length == 0 || op > 8 {
            return Err(ModificationDecodeError::InvalidCigar(format!(
                "unsupported operation code {op} or zero length"
            )));
        }
        let consumes_query = matches!(op, 0 | 1 | 4 | 7 | 8);
        let consumes_reference = matches!(op, 0 | 2 | 3 | 7 | 8);
        if consumes_query {
            let end = query.checked_add(length).ok_or_else(|| {
                ModificationDecodeError::InvalidCigar("query span overflow".to_string())
            })?;
            while target < positions.len() && positions[target] < end {
                if positions[target] >= query && consumes_reference {
                    projected[target] = Some(reference + (positions[target] - query) as i64);
                }
                target += 1;
            }
            query = end;
        }
        if consumes_reference {
            reference = reference.checked_add(length as i64).ok_or_else(|| {
                ModificationDecodeError::InvalidCigar("reference span overflow".to_string())
            })?;
        }
    }
    if query != sequence_length {
        return Err(ModificationDecodeError::InvalidCigar(format!(
            "query span {query} does not equal sequence length {sequence_length}"
        )));
    }
    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bam::{records::RecordLayout, write::serialize_record_layout};

    fn packed(sequence: &str) -> Vec<u8> {
        let code = |base| match base {
            b'A' => 1,
            b'C' => 2,
            b'G' => 4,
            b'T' => 8,
            _ => 15,
        };
        sequence
            .as_bytes()
            .chunks(2)
            .map(|chunk| (code(chunk[0]) << 4) | chunk.get(1).copied().map(code).unwrap_or(0))
            .collect()
    }

    fn cigar(ops: &[(usize, u8)]) -> Vec<u8> {
        ops.iter()
            .flat_map(|(length, op)| (((*length as u32) << 4) | u32::from(*op)).to_le_bytes())
            .collect()
    }

    fn aux(mm: Option<&str>, ml: Option<&[u8]>, mn: Option<i32>) -> Vec<u8> {
        let mut bytes = Vec::new();
        if let Some(mm) = mm {
            bytes.extend_from_slice(b"MMZ");
            bytes.extend_from_slice(mm.as_bytes());
            bytes.push(0);
        }
        if let Some(ml) = ml {
            bytes.extend_from_slice(b"MLBC");
            bytes.extend_from_slice(&(ml.len() as i32).to_le_bytes());
            bytes.extend_from_slice(ml);
        }
        if let Some(mn) = mn {
            bytes.extend_from_slice(b"MNi");
            bytes.extend_from_slice(&mn.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn projects_matches_soft_clips_insertions_and_deletions() {
        let sequence = "CCACCCCCC";
        let result = decode_c_m_modifications(
            &aux(Some("C+m,0,0,0,0,0,0,0,0;"), Some(&[1; 8]), Some(9)),
            &packed(sequence),
            9,
            &cigar(&[(1, 4), (2, 0), (1, 1), (2, 0), (2, 2), (3, 0)]),
            100,
        )
        .unwrap()
        .unwrap();
        let refs: Vec<_> = result
            .modifications
            .iter()
            .map(|call| call.reference_position)
            .collect();
        assert_eq!(
            refs,
            vec![
                None,
                Some(100),
                None,
                Some(102),
                Some(103),
                Some(106),
                Some(107),
                Some(108)
            ]
        );
    }

    #[test]
    fn mm_deltas_count_only_canonical_cytosines() {
        let result = decode_c_m_modifications(
            &aux(Some("C+m,1,0;"), Some(&[20, 30]), Some(6)),
            &packed("ACGCCC"),
            6,
            &cigar(&[(6, 0)]),
            10,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            result
                .modifications
                .iter()
                .map(|call| (call.query_position, call.reference_position))
                .collect::<Vec<_>>(),
            vec![(3, Some(13)), (4, Some(14))]
        );
    }

    #[test]
    fn reverse_oriented_seq_is_not_flipped_before_projection() {
        // A reverse BAM record stores SEQ in alignment orientation. Its C calls
        // therefore project left-to-right through CIGAR exactly as encoded.
        let result = decode_c_m_modifications(
            &aux(Some("C+m,0,1;"), Some(&[7, 9]), Some(5)),
            &packed("CCACC"),
            5,
            &cigar(&[(5, 0)]),
            200,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            result
                .modifications
                .iter()
                .map(|call| call.reference_position)
                .collect::<Vec<_>>(),
            vec![Some(200), Some(203)]
        );
    }

    #[test]
    fn record_view_entry_point_preserves_reverse_seq_orientation() {
        let sequence = "CCACC";
        let layout = RecordLayout {
            block_size: 0,
            ref_id: 0,
            pos: 200,
            bin: 0,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags: 0x10,
            mapping_quality: 60,
            n_cigar_op: 1,
            l_seq: sequence.len(),
            read_name: "reverse".to_string(),
            cigar_bytes: cigar(&[(5, 0)]),
            sequence_bytes: packed(sequence),
            quality_bytes: vec![30; sequence.len()],
            aux_bytes: aux(Some("C+m,0,1;"), Some(&[7, 9]), Some(5)),
        };
        let raw = serialize_record_layout(&layout);
        let record = BamRecordView::parse(&raw).unwrap();

        let result = decode_record_c_m_modifications(&record).unwrap().unwrap();
        assert!(record.flag_summary().is_reverse);
        assert_eq!(
            result
                .modifications
                .iter()
                .map(|call| call.reference_position)
                .collect::<Vec<_>>(),
            vec![Some(200), Some(203)]
        );
    }

    #[test]
    fn accepts_empty_mm_and_ml_arrays() {
        let result = decode_c_m_modifications(
            &aux(Some("C+m;"), Some(&[]), Some(2)),
            &packed("AC"),
            2,
            &cigar(&[(2, 0)]),
            0,
        )
        .unwrap()
        .unwrap();
        assert!(result.modifications.is_empty());
    }

    #[test]
    fn distinguishes_absent_and_partial_trios() {
        assert_eq!(
            decode_c_m_modifications(&[], &packed("C"), 1, &cigar(&[(1, 0)]), 0).unwrap(),
            None
        );
        assert_eq!(
            decode_c_m_modifications(
                &aux(Some("C+m,0;"), None, Some(1)),
                &packed("C"),
                1,
                &cigar(&[(1, 0)]),
                0
            ),
            Err(ModificationDecodeError::PartialTrio)
        );
    }

    #[test]
    fn rejects_mn_mismatch_and_ml_cardinality() {
        let mismatch = decode_c_m_modifications(
            &aux(Some("C+m,0;"), Some(&[1]), Some(2)),
            &packed("C"),
            1,
            &cigar(&[(1, 0)]),
            0,
        );
        assert!(matches!(
            mismatch,
            Err(ModificationDecodeError::MnMismatch { .. })
        ));
        let cardinality = decode_c_m_modifications(
            &aux(Some("C+m,0;"), Some(&[]), Some(1)),
            &packed("C"),
            1,
            &cigar(&[(1, 0)]),
            0,
        );
        assert!(matches!(
            cardinality,
            Err(ModificationDecodeError::MlCardinality { .. })
        ));
    }

    #[test]
    fn rejects_unsupported_codes_and_malformed_deltas() {
        for mm in ["A+a,0;", "C+h,0;", "C-m,0;", "C+m,0;C+h,0;"] {
            assert!(matches!(
                decode_c_m_modifications(
                    &aux(Some(mm), Some(&[1]), Some(1)),
                    &packed("C"),
                    1,
                    &cigar(&[(1, 0)]),
                    0
                ),
                Err(ModificationDecodeError::UnsupportedMmCode(_))
            ));
        }
        for mm in ["C+m,-1;", "C+m,x;", "C+m,1;"] {
            assert!(matches!(
                decode_c_m_modifications(
                    &aux(Some(mm), Some(&[1]), Some(1)),
                    &packed("C"),
                    1,
                    &cigar(&[(1, 0)]),
                    0
                ),
                Err(ModificationDecodeError::MalformedMm(_))
            ));
        }
    }

    #[test]
    fn rejects_duplicate_tags_wrong_types_and_invalid_cigar() {
        let mut duplicate = aux(Some("C+m,0;"), Some(&[1]), Some(1));
        duplicate.extend_from_slice(b"MNi\x01\0\0\0");
        assert_eq!(
            decode_c_m_modifications(&duplicate, &packed("C"), 1, &cigar(&[(1, 0)]), 0),
            Err(ModificationDecodeError::DuplicateTag(*b"MN"))
        );

        let wrong_ml = b"MMZC+m,0;\0MLBi\x01\0\0\0\x01\0\0\0MNi\x01\0\0\0";
        assert!(matches!(
            decode_c_m_modifications(wrong_ml, &packed("C"), 1, &cigar(&[(1, 0)]), 0),
            Err(ModificationDecodeError::MalformedAux(_))
        ));

        assert!(matches!(
            decode_c_m_modifications(
                &aux(Some("C+m,0;"), Some(&[1]), Some(1)),
                &packed("C"),
                1,
                &cigar(&[(1, 2)]),
                0
            ),
            Err(ModificationDecodeError::InvalidCigar(_))
        ));
    }
}
