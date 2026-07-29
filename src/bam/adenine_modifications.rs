//! Strict projection of ONT `A+a` accessibility calls.

use super::modifications::{
    MmSkippedBaseMode, ModificationDecodeError, decode_target_modifications, groups::TargetGroup,
};
use super::record::BamRecordView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdenineModification {
    pub query_position: usize,
    pub reference_position: Option<i64>,
    pub probability: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmittedCanonicalAdenine {
    pub query_position: usize,
    pub reference_position: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdenineModificationTrio {
    pub sequence_length: usize,
    pub skipped_base_mode: MmSkippedBaseMode,
    pub modifications: Vec<AdenineModification>,
    pub callable_omitted_adenines: Option<Vec<OmittedCanonicalAdenine>>,
}

pub fn decode_record_a_a_modifications(
    record: &BamRecordView<'_>,
) -> Result<Option<AdenineModificationTrio>, ModificationDecodeError> {
    if record.flag_summary().is_unmapped || record.ref_id() < 0 || record.pos() < 0 {
        return Err(ModificationDecodeError::UnmappedRecord);
    }
    decode_a_a_modifications(
        record.aux_bytes(),
        record.sequence_bytes(),
        record.sequence_len(),
        record.cigar_bytes(),
        i64::from(record.pos()),
        record.flag_summary().is_reverse,
    )
}

/// Decode and project the exact ONT accessibility group `A+a`.
pub fn decode_a_a_modifications(
    aux: &[u8],
    packed_sequence: &[u8],
    sequence_length: usize,
    cigar: &[u8],
    reference_start: i64,
    is_reverse_complemented: bool,
) -> Result<Option<AdenineModificationTrio>, ModificationDecodeError> {
    Ok(decode_target_modifications(
        aux,
        packed_sequence,
        sequence_length,
        cigar,
        reference_start,
        is_reverse_complemented,
        TargetGroup::Aa,
    )?
    .map(|trio| AdenineModificationTrio {
        sequence_length: trio.sequence_length,
        skipped_base_mode: trio.skipped_base_mode,
        modifications: trio
            .modifications
            .into_iter()
            .map(|call| AdenineModification {
                query_position: call.query_position,
                reference_position: call.reference_position,
                probability: call.probability,
            })
            .collect(),
        callable_omitted_adenines: trio.callable_omitted.map(|positions| {
            positions
                .into_iter()
                .map(|position| OmittedCanonicalAdenine {
                    query_position: position.query_position,
                    reference_position: position.reference_position,
                })
                .collect()
        }),
    }))
}
