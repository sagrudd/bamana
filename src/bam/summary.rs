use std::collections::{BTreeMap, HashSet};

use crate::bam::records::LightAlignmentRecord;

#[derive(Debug, Clone, Default)]
pub struct SummarySnapshot {
    pub records_examined: u64,
    pub mapped_records: u64,
    pub unmapped_records: u64,
    pub primary_records: u64,
    pub secondary_records: u64,
    pub supplementary_records: u64,
    pub duplicate_records: u64,
    pub qc_fail_records: u64,
    pub paired_records: u64,
    pub properly_paired_records: u64,
    pub read1_records: u64,
    pub read2_records: u64,
    pub reverse_strand_records: u64,
    pub contradictory_mapping_state_records: u64,
    pub references_with_mapped_reads_observed: usize,
    pub mapped_reference_ids: HashSet<usize>,
    pub mapq_min: Option<u8>,
    pub mapq_max: Option<u8>,
    pub mapq_sum: u64,
    pub mapq_zero_count: u64,
    pub mapq_histogram: Option<BTreeMap<u8, u64>>,
}

#[derive(Debug, Clone)]
pub struct SummaryAccumulator {
    snapshot: SummarySnapshot,
    include_mapq_hist: bool,
}

impl SummaryAccumulator {
    pub fn new(include_mapq_hist: bool) -> Self {
        Self {
            snapshot: SummarySnapshot {
                mapq_histogram: include_mapq_hist.then(BTreeMap::new),
                ..SummarySnapshot::default()
            },
            include_mapq_hist,
        }
    }

    pub fn observe(&mut self, record: &LightAlignmentRecord) {
        self.snapshot.records_examined += 1;

        if record.is_secondary {
            self.snapshot.secondary_records += 1;
        } else if record.is_supplementary {
            self.snapshot.supplementary_records += 1;
        } else {
            self.snapshot.primary_records += 1;
        }

        if record.is_duplicate {
            self.snapshot.duplicate_records += 1;
        }
        if record.is_qc_fail {
            self.snapshot.qc_fail_records += 1;
        }
        if record.is_paired {
            self.snapshot.paired_records += 1;
        }
        if record.is_proper_pair {
            self.snapshot.properly_paired_records += 1;
        }
        if record.is_read1 {
            self.snapshot.read1_records += 1;
        }
        if record.is_read2 {
            self.snapshot.read2_records += 1;
        }
        if record.is_reverse {
            self.snapshot.reverse_strand_records += 1;
        }

        let contradictory_mapping_state = (record.ref_id >= 0 && record.is_unmapped)
            || (record.ref_id < 0 && !record.is_unmapped);
        if contradictory_mapping_state {
            self.snapshot.contradictory_mapping_state_records += 1;
        }

        if record.ref_id >= 0 && !record.is_unmapped {
            self.snapshot.mapped_records += 1;
            if let Ok(ref_index) = usize::try_from(record.ref_id) {
                self.snapshot.mapped_reference_ids.insert(ref_index);
            }
        } else {
            self.snapshot.unmapped_records += 1;
        }

        self.snapshot.references_with_mapped_reads_observed =
            self.snapshot.mapped_reference_ids.len();

        let mapq = record.mapping_quality;
        self.snapshot.mapq_min = Some(
            self.snapshot
                .mapq_min
                .map_or(mapq, |current| current.min(mapq)),
        );
        self.snapshot.mapq_max = Some(
            self.snapshot
                .mapq_max
                .map_or(mapq, |current| current.max(mapq)),
        );
        self.snapshot.mapq_sum += u64::from(mapq);
        if mapq == 0 {
            self.snapshot.mapq_zero_count += 1;
        }
        if self.include_mapq_hist {
            if let Some(histogram) = self.snapshot.mapq_histogram.as_mut() {
                *histogram.entry(mapq).or_insert(0) += 1;
            }
        }
    }

    pub fn snapshot(&self) -> SummarySnapshot {
        self.snapshot.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::SummaryAccumulator;
    use crate::bam::records::LightAlignmentRecord;

    #[test]
    fn accumulates_basic_counts() {
        let mut accumulator = SummaryAccumulator::new(true);
        accumulator.observe(&LightAlignmentRecord {
            ref_id: 0,
            pos: 10,
            flags: 0x43,
            mapping_quality: 60,
            read_name: "read1".to_string(),
            is_unmapped: false,
            is_paired: true,
            is_proper_pair: true,
            is_reverse: false,
            is_secondary: false,
            is_supplementary: false,
            is_qc_fail: false,
            is_duplicate: false,
            is_read1: true,
            is_read2: false,
        });
        accumulator.observe(&LightAlignmentRecord {
            ref_id: -1,
            pos: -1,
            flags: 0x4 | 0x80,
            mapping_quality: 0,
            read_name: "read2".to_string(),
            is_unmapped: true,
            is_paired: false,
            is_proper_pair: false,
            is_reverse: false,
            is_secondary: false,
            is_supplementary: false,
            is_qc_fail: false,
            is_duplicate: false,
            is_read1: false,
            is_read2: true,
        });

        let snapshot = accumulator.snapshot();
        assert_eq!(snapshot.records_examined, 2);
        assert_eq!(snapshot.mapped_records, 1);
        assert_eq!(snapshot.unmapped_records, 1);
        assert_eq!(snapshot.read1_records, 1);
        assert_eq!(snapshot.read2_records, 1);
        assert_eq!(snapshot.mapq_zero_count, 1);
        assert_eq!(snapshot.references_with_mapped_reads_observed, 1);
        assert_eq!(
            snapshot
                .mapq_histogram
                .as_ref()
                .and_then(|histogram| histogram.get(&60))
                .copied(),
            Some(1)
        );
    }
}
