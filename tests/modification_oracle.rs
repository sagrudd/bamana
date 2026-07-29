//! Test-only maintained-parser oracle for reverse-strand SAM MM orientation.

use bamana::bam::modifications::decode_c_m_modifications;
use noodles_sam::{alignment::record_buf::Sequence, record::data::field::value::BaseModifications};

#[test]
fn native_reverse_positions_match_noodles_sam_oracle() {
    let sequence = "CACCCGATGACCGGCT";
    let mm = "C+m,1,0,0;";
    let oracle = BaseModifications::parse(mm, true, &Sequence::from(sequence.as_bytes())).unwrap();
    let native = decode_c_m_modifications(
        &aux(mm, &[10, 20, 30], sequence.len()),
        &packed(sequence),
        sequence.len(),
        &cigar(sequence.len()),
        100,
        true,
    )
    .unwrap()
    .unwrap();

    let mut native_positions: Vec<_> = native
        .modifications
        .iter()
        .map(|call| call.query_position)
        .collect();
    native_positions.sort_unstable_by(|left, right| right.cmp(left));
    assert_eq!(native_positions, oracle.as_ref()[0].positions());
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

fn cigar(length: usize) -> Vec<u8> {
    (((length as u32) << 4).to_le_bytes()).to_vec()
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
