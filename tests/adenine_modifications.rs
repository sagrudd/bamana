use bamana::bam::{
    adenine_modifications::decode_a_a_modifications, modifications::ModificationDecodeError,
};
use noodles_sam::{alignment::record_buf::Sequence, record::data::field::value::BaseModifications};

#[test]
fn public_a_a_boundary_is_documented() {
    let docs = include_str!("../docs/sphinx/native_bam_record_scanner.rst");
    assert!(docs.contains("decode_a_a_modifications"));
    assert!(docs.contains("A+a"));
    assert!(docs.contains("complete ML cardinality"));
}

#[test]
fn projects_forward_calls_across_insertion_and_deletion() {
    let result = decode_a_a_modifications(
        &aux("C+h.,0;A+a,0,1;", &[3, 7, 9], 5),
        &packed("AAACA"),
        5,
        &cigar(&[(1, 0), (1, 1), (2, 0), (1, 2), (1, 0)]),
        100,
        false,
    )
    .unwrap()
    .unwrap();
    assert_eq!(calls(&result), vec![(0, Some(100), 7), (2, Some(101), 9)]);
}

#[test]
fn reverse_calls_count_complemented_adenines_right_to_left() {
    let result = decode_a_a_modifications(
        &aux("A+a,0,1;", &[7, 9], 5),
        &packed("TTTGT"),
        5,
        &cigar(&[(5, 0)]),
        200,
        true,
    )
    .unwrap()
    .unwrap();
    assert_eq!(calls(&result), vec![(1, Some(201), 9), (4, Some(204), 7)]);
    let oracle =
        BaseModifications::parse("A+a,0,1;", true, &Sequence::from(b"TTTGT".as_slice())).unwrap();
    let mut native_positions: Vec<_> = result
        .modifications
        .iter()
        .map(|call| call.query_position)
        .collect();
    native_positions.sort_unstable_by(|left, right| right.cmp(left));
    assert_eq!(native_positions, oracle.as_ref()[0].positions());
}

#[test]
fn valid_non_target_groups_are_absent_after_full_ml_validation() {
    let valid = aux("C+h.,0;C+m.,0;", &[3, 4], 2);
    assert_eq!(
        decode_a_a_modifications(&valid, &packed("AC"), 2, &cigar(&[(2, 0)]), 0, false).unwrap(),
        None
    );

    let wrong_ml = aux("C+h.,0;C+m.,0;", &[3], 2);
    assert_eq!(
        decode_a_a_modifications(&wrong_ml, &packed("AC"), 2, &cigar(&[(2, 0)]), 0, false),
        Err(ModificationDecodeError::MlCardinality {
            positions: 2,
            probabilities: 1,
        })
    );
}

#[test]
fn malformed_and_duplicate_target_groups_fail_without_payload_echo() {
    let malformed = aux("A+a,SENSITIVE;", &[1], 1);
    let error = decode_a_a_modifications(&malformed, &packed("A"), 1, &cigar(&[(1, 0)]), 0, false)
        .unwrap_err();
    assert!(matches!(error, ModificationDecodeError::MalformedMm(_)));
    assert!(!error.to_string().contains("SENSITIVE"));

    let duplicate = aux("A+a,0;C+m,0;A+a.,0;", &[1, 2, 3], 2);
    assert_eq!(
        decode_a_a_modifications(&duplicate, &packed("AC"), 2, &cigar(&[(2, 0)]), 0, false),
        Err(ModificationDecodeError::DuplicateAaGroup)
    );
}

fn calls(
    result: &bamana::bam::adenine_modifications::AdenineModificationTrio,
) -> Vec<(usize, Option<i64>, u8)> {
    result
        .modifications
        .iter()
        .map(|call| {
            (
                call.query_position,
                call.reference_position,
                call.probability,
            )
        })
        .collect()
}

fn aux(mm: &str, ml: &[u8], sequence_length: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MMZ");
    bytes.extend_from_slice(mm.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(b"MLBC");
    bytes.extend_from_slice(&(ml.len() as i32).to_le_bytes());
    bytes.extend_from_slice(ml);
    bytes.extend_from_slice(b"MNi");
    bytes.extend_from_slice(&(sequence_length as i32).to_le_bytes());
    bytes
}

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
