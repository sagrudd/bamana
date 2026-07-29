use super::{MmSkippedBaseMode, ModificationDecodeError, bam_base};

pub(super) struct DecodedMm {
    pub(super) selected: Option<SelectedCm>,
    pub(super) total_ml_values: usize,
}

pub(super) struct SelectedCm {
    pub(super) skipped_base_mode: MmSkippedBaseMode,
    pub(super) canonical_positions: Vec<usize>,
    pub(super) called_positions: Vec<usize>,
    pub(super) selected_ml_offset: usize,
}

struct ParsedGroup {
    canonical_base: u8,
    strand: u8,
    codes: String,
    skipped_base_mode: MmSkippedBaseMode,
    called_positions: Vec<usize>,
    canonical_positions: Vec<usize>,
    ml_values: usize,
}

pub(super) fn decode_mm_groups(
    mm: &str,
    packed_sequence: &[u8],
    sequence_length: usize,
    is_reverse_complemented: bool,
) -> Result<DecodedMm, ModificationDecodeError> {
    if !mm.ends_with(';') {
        return Err(malformed("missing final group terminator"));
    }
    let groups = &mm[..mm.len() - 1];
    if groups.is_empty() {
        return Err(malformed("MM contains no groups"));
    }

    let mut selected = None;
    let mut ml_offset = 0_usize;
    for (group_index, encoded) in groups.split(';').enumerate() {
        if encoded.is_empty() {
            return Err(malformed_at(group_index, "empty group"));
        }
        let group = parse_group(
            encoded,
            packed_sequence,
            sequence_length,
            is_reverse_complemented,
            group_index,
        )?;
        let is_selected =
            group.canonical_base == b'C' && group.strand == b'+' && group.codes == "m";
        if is_selected {
            if selected.is_some() {
                return Err(ModificationDecodeError::DuplicateCmGroup);
            }
            selected = Some((
                group.skipped_base_mode,
                group.canonical_positions,
                group.called_positions,
                ml_offset,
            ));
        }
        ml_offset = ml_offset
            .checked_add(group.ml_values)
            .ok_or_else(|| malformed_at(group_index, "ML cardinality overflow"))?;
    }
    Ok(DecodedMm {
        selected: selected.map(
            |(skipped_base_mode, canonical_positions, called_positions, selected_ml_offset)| {
                SelectedCm {
                    skipped_base_mode,
                    canonical_positions,
                    called_positions,
                    selected_ml_offset,
                }
            },
        ),
        total_ml_values: ml_offset,
    })
}

fn parse_group(
    encoded: &str,
    packed_sequence: &[u8],
    sequence_length: usize,
    is_reverse_complemented: bool,
    group_index: usize,
) -> Result<ParsedGroup, ModificationDecodeError> {
    let (header, deltas) = encoded.split_once(',').unwrap_or((encoded, ""));
    if encoded.ends_with(',') {
        return Err(malformed_at(
            group_index,
            "empty delta list after separator",
        ));
    }
    let bytes = header.as_bytes();
    if bytes.len() < 3 || !matches!(bytes[0], b'A' | b'C' | b'G' | b'T' | b'U' | b'N') {
        return Err(malformed_at(
            group_index,
            "invalid canonical base or short header",
        ));
    }
    if !matches!(bytes[1], b'+' | b'-') {
        return Err(malformed_at(group_index, "invalid modification strand"));
    }
    let (code_end, skipped_base_mode) = match bytes.last() {
        Some(b'.') => (bytes.len() - 1, MmSkippedBaseMode::ExplicitCanonical),
        Some(b'?') => (bytes.len() - 1, MmSkippedBaseMode::Unknown),
        _ => (bytes.len(), MmSkippedBaseMode::DefaultCanonical),
    };
    let codes = &header[2..code_end];
    let code_count = modification_code_count(codes)
        .ok_or_else(|| malformed_at(group_index, "unsupported modification-code syntax"))?;
    let canonical_positions = canonical_positions(
        bytes[0],
        packed_sequence,
        sequence_length,
        is_reverse_complemented,
        group_index,
    )?;
    let mut delta_positions = canonical_positions.clone();
    if is_reverse_complemented {
        delta_positions.reverse();
    }
    let called_positions = decode_deltas(deltas, &delta_positions, group_index)?;
    let ml_values = called_positions
        .len()
        .checked_mul(code_count)
        .ok_or_else(|| malformed_at(group_index, "ML cardinality overflow"))?;
    Ok(ParsedGroup {
        canonical_base: bytes[0],
        strand: bytes[1],
        codes: codes.to_string(),
        skipped_base_mode,
        called_positions,
        canonical_positions,
        ml_values,
    })
}

fn modification_code_count(codes: &str) -> Option<usize> {
    if codes.is_empty() {
        None
    } else if codes.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        Some(codes.len())
    } else if codes.bytes().all(|byte| byte.is_ascii_digit()) {
        Some(1)
    } else {
        None
    }
}

fn canonical_positions(
    canonical_base: u8,
    packed_sequence: &[u8],
    sequence_length: usize,
    is_reverse_complemented: bool,
    group_index: usize,
) -> Result<Vec<usize>, ModificationDecodeError> {
    let forward_code = match canonical_base {
        b'A' => Some(1),
        b'C' => Some(2),
        b'G' => Some(4),
        b'T' | b'U' => Some(8),
        b'N' => None,
        _ => return Err(malformed_at(group_index, "unsupported canonical base")),
    };
    let expected = if is_reverse_complemented {
        forward_code.map(complement_code)
    } else {
        forward_code
    };
    Ok((0..sequence_length)
        .filter(|position| expected.is_none_or(|code| bam_base(packed_sequence, *position) == code))
        .collect())
}

fn complement_code(code: u8) -> u8 {
    match code {
        1 => 8,
        2 => 4,
        4 => 2,
        8 => 1,
        _ => code,
    }
}

fn decode_deltas(
    deltas: &str,
    canonical_positions: &[usize],
    group_index: usize,
) -> Result<Vec<usize>, ModificationDecodeError> {
    if deltas.is_empty() {
        return Ok(Vec::new());
    }
    let mut canonical_index = 0_usize;
    let mut called_positions = Vec::new();
    for delta in deltas.split(',') {
        if delta.is_empty() || !delta.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(malformed_at(group_index, "non-decimal delta"));
        }
        let skipped = delta
            .parse::<usize>()
            .map_err(|_| malformed_at(group_index, "delta overflow"))?;
        canonical_index = canonical_index
            .checked_add(skipped)
            .ok_or_else(|| malformed_at(group_index, "canonical-base index overflow"))?;
        let position = canonical_positions
            .get(canonical_index)
            .ok_or_else(|| malformed_at(group_index, "delta exceeds canonical bases"))?;
        called_positions.push(*position);
        canonical_index += 1;
    }
    Ok(called_positions)
}

fn malformed(detail: &'static str) -> ModificationDecodeError {
    ModificationDecodeError::MalformedMm(detail.to_string())
}

fn malformed_at(group_index: usize, detail: &'static str) -> ModificationDecodeError {
    ModificationDecodeError::MalformedMm(format!("group {}: {detail}", group_index + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn accounts_for_observed_group_orders() {
        for mm in ["A+a.,0;C+h.,0;C+m.,0;", "C+h.,0;C+m.,0;A+a.,0;"] {
            let decoded = decode_mm_groups(mm, &packed("AC"), 2, false).unwrap();
            assert_eq!(decoded.total_ml_values, 3);
            assert_eq!(decoded.selected.unwrap().called_positions, vec![1]);
        }
    }

    #[test]
    fn multi_code_group_consumes_one_probability_per_code_and_call() {
        let decoded = decode_mm_groups("A+az,0,0;C+m,0;G+h,0;", &packed("AACG"), 4, false).unwrap();
        assert_eq!(decoded.selected.unwrap().selected_ml_offset, 4);
        assert_eq!(decoded.total_ml_values, 6);
    }

    #[test]
    fn reverse_deltas_match_maintained_sam_oracle_positions() {
        let sequence = "CACCCGATGACCGGCT";
        let mm = "C+m,1,0,0;";
        let decoded = decode_mm_groups(mm, &packed(sequence), sequence.len(), true).unwrap();
        assert_eq!(decoded.selected.unwrap().called_positions, vec![12, 8, 5]);
    }

    #[test]
    fn accepts_valid_groups_without_target_while_retaining_ml_cardinality() {
        let decoded = decode_mm_groups("A+a.,0;C+h.,0;", &packed("AC"), 2, false).unwrap();
        assert!(decoded.selected.is_none());
        assert_eq!(decoded.total_ml_values, 2);
    }

    #[test]
    fn rejects_duplicate_selected_group_and_ambiguous_syntax() {
        assert_eq!(
            decode_mm_groups("C+m,0;A+a,0;C+m.,0;", &packed("AC"), 2, false)
                .err()
                .unwrap(),
            ModificationDecodeError::DuplicateCmGroup
        );
        for mm in [
            "A+a/,0;C+m,0;",
            "A+,0;C+m,0;",
            "A+a,x;C+m,0;",
            "A+a,;C+m,0;",
        ] {
            assert!(matches!(
                decode_mm_groups(mm, &packed("AC"), 2, false),
                Err(ModificationDecodeError::MalformedMm(_))
            ));
        }
    }
}
