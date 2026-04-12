use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::{
    bam::{
        header::{HeaderPayload, parse_bam_header_from_reader},
        reader::BamReader,
        records::{RecordLayout, read_next_record_layout},
        tags::traverse_aux_fields,
    },
    bgzf,
    error::AppError,
};

#[derive(Debug, Clone, Copy)]
pub struct ValidationOptions {
    pub max_errors: usize,
    pub max_warnings: usize,
    pub header_only: bool,
    pub record_limit: Option<u64>,
    pub fail_fast: bool,
    pub include_warnings: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationMode {
    HeaderOnly,
    BoundedRecords,
    Full,
}

#[derive(Debug, Serialize)]
pub struct ValidatePayload {
    pub format: &'static str,
    pub mode: ValidationMode,
    pub valid: bool,
    pub summary: ValidationSummary,
    pub findings: Vec<ValidationFinding>,
    pub semantic_note: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationSummary {
    pub header_valid: bool,
    pub records_examined: u64,
    pub full_file_examined: bool,
    pub errors: u64,
    pub warnings: u64,
    pub infos: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationFinding {
    pub severity: FindingSeverity,
    pub scope: FindingScope,
    pub code: String,
    pub message: String,
    pub record_index: Option<u64>,
    pub reference_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingScope {
    File,
    Header,
    Record,
    Aux,
}

pub fn validate_bam(
    path: &std::path::Path,
    options: ValidationOptions,
) -> Result<ValidatePayload, AppError> {
    let mut reader = BamReader::open(path)?;
    let mode = if options.header_only {
        ValidationMode::HeaderOnly
    } else if options.record_limit.is_some() {
        ValidationMode::BoundedRecords
    } else {
        ValidationMode::Full
    };

    let mut collector = FindingCollector::new(options);

    match bgzf::has_bgzf_eof(path) {
        Ok(true) => collector.push(ValidationFinding {
            severity: FindingSeverity::Info,
            scope: FindingScope::File,
            code: "bgzf_eof_present".to_string(),
            message: "Canonical BGZF EOF marker was present at the end of the file.".to_string(),
            record_index: None,
            reference_name: None,
            tag: None,
        }),
        Ok(false) => collector.push(ValidationFinding {
            severity: FindingSeverity::Warning,
            scope: FindingScope::File,
            code: "missing_bgzf_eof".to_string(),
            message: "Canonical BGZF EOF marker was not present at the end of the file."
                .to_string(),
            record_index: None,
            reference_name: None,
            tag: None,
        }),
        Err(_) => {}
    }

    let header = match parse_bam_header_from_reader(&mut reader) {
        Ok(header) => header,
        Err(error) => {
            collector.push(ValidationFinding {
                severity: FindingSeverity::Error,
                scope: FindingScope::Header,
                code: "invalid_header".to_string(),
                message: error
                    .to_json_error()
                    .detail
                    .unwrap_or_else(|| "BAM header could not be parsed.".to_string()),
                record_index: None,
                reference_name: None,
                tag: None,
            });

            return Ok(build_payload(mode, false, 0, false, collector));
        }
    };

    validate_header(&header, &mut collector);

    if options.header_only || collector.should_stop() {
        return Ok(build_payload(
            mode,
            collector.error_count == 0,
            0,
            false,
            collector,
        ));
    }

    let reference_names = header_reference_names(&header);
    let mut records_examined = 0_u64;
    let mut full_file_examined = false;

    loop {
        if let Some(limit) = options.record_limit {
            if records_examined >= limit {
                break;
            }
        }

        match read_next_record_layout(&mut reader) {
            Ok(Some(record)) => {
                records_examined += 1;
                validate_record(&record, records_examined, &reference_names, &mut collector);
                if collector.should_stop() {
                    break;
                }
            }
            Ok(None) => {
                full_file_examined = true;
                break;
            }
            Err(AppError::TruncatedFile { detail, .. }) => {
                collector.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    scope: FindingScope::Record,
                    code: "truncated_record".to_string(),
                    message: detail,
                    record_index: Some(records_examined + 1),
                    reference_name: None,
                    tag: None,
                });
                break;
            }
            Err(AppError::InvalidRecord { detail, .. }) => {
                collector.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    scope: FindingScope::Record,
                    code: "invalid_record".to_string(),
                    message: detail,
                    record_index: Some(records_examined + 1),
                    reference_name: None,
                    tag: None,
                });
                break;
            }
            Err(error) => return Err(error),
        }
    }

    let valid = collector.error_count == 0;
    Ok(build_payload(
        mode,
        valid,
        records_examined,
        full_file_examined,
        collector,
    ))
}

fn validate_header(header: &HeaderPayload, collector: &mut FindingCollector) {
    let mut binary_names = HashSet::new();
    for reference in &header.header.references {
        if reference.name.is_empty() {
            collector.push(ValidationFinding {
                severity: FindingSeverity::Error,
                scope: FindingScope::Header,
                code: "empty_reference_name".to_string(),
                message: "Binary reference dictionary contains an empty reference name."
                    .to_string(),
                record_index: None,
                reference_name: None,
                tag: None,
            });
        }

        if !binary_names.insert(reference.name.clone()) {
            collector.push(ValidationFinding {
                severity: FindingSeverity::Error,
                scope: FindingScope::Header,
                code: "duplicate_reference_name".to_string(),
                message: format!(
                    "Binary reference dictionary contains duplicate reference name {}.",
                    reference.name
                ),
                record_index: None,
                reference_name: Some(reference.name.clone()),
                tag: None,
            });
        }

        if let Some(text_length) = reference.text_header_length {
            collector.push(ValidationFinding {
                severity: FindingSeverity::Warning,
                scope: FindingScope::Header,
                code: "reference_length_mismatch".to_string(),
                message: format!(
                    "Binary reference length {} disagrees with textual @SQ LN {} for {}.",
                    reference.length, text_length, reference.name
                ),
                record_index: None,
                reference_name: Some(reference.name.clone()),
                tag: None,
            });
        }
    }

    let mut textual_sq = parse_textual_sq(&header.header.raw_header_text);
    for reference in &header.header.references {
        textual_sq.remove(&reference.name);
    }
    for name in textual_sq.keys() {
        collector.push(ValidationFinding {
            severity: FindingSeverity::Warning,
            scope: FindingScope::Header,
            code: "textual_sq_not_in_binary_dictionary".to_string(),
            message: format!(
                "Textual header contains @SQ entry {} that is not present in the binary reference dictionary.",
                name
            ),
            record_index: None,
            reference_name: Some(name.clone()),
            tag: None,
        });
    }

    let mut pg_ids = HashSet::new();
    for program in &header.header.programs {
        if let Some(id) = &program.id {
            if !pg_ids.insert(id.clone()) {
                collector.push(ValidationFinding {
                    severity: FindingSeverity::Warning,
                    scope: FindingScope::Header,
                    code: "duplicate_program_id".to_string(),
                    message: "Header contains duplicate @PG ID values.".to_string(),
                    record_index: None,
                    reference_name: None,
                    tag: None,
                });
            }
        }
    }

    let mut rg_ids = HashSet::new();
    for read_group in &header.header.read_groups {
        if let Some(id) = &read_group.id {
            if !rg_ids.insert(id.clone()) {
                collector.push(ValidationFinding {
                    severity: FindingSeverity::Warning,
                    scope: FindingScope::Header,
                    code: "duplicate_read_group_id".to_string(),
                    message: format!("Header contains duplicate @RG ID value {}.", id),
                    record_index: None,
                    reference_name: None,
                    tag: None,
                });
            }
        }
    }
}

fn validate_record(
    record: &RecordLayout,
    record_index: u64,
    reference_names: &[String],
    collector: &mut FindingCollector,
) {
    let reference_name = usize::try_from(record.ref_id)
        .ok()
        .and_then(|index| reference_names.get(index))
        .cloned();

    if record.block_size < 32 {
        collector.push(record_finding(
            FindingSeverity::Error,
            "invalid_block_size",
            "Record block size was smaller than the BAM core section.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }

    if record.read_name.is_empty() {
        collector.push(record_finding(
            FindingSeverity::Error,
            "empty_read_name",
            "Alignment record read name was empty.",
            record_index,
            reference_name.clone(),
            None,
        ));
    } else if record
        .read_name
        .chars()
        .any(|character| character.is_control())
    {
        collector.push(record_finding(
            FindingSeverity::Warning,
            "control_character_in_read_name",
            "Read name contains embedded control characters.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }

    let is_unmapped = record.flags & 0x4 != 0;
    let is_paired = record.flags & 0x1 != 0;
    let is_proper_pair = record.flags & 0x2 != 0;
    let is_secondary = record.flags & 0x100 != 0;
    let is_qc_fail = record.flags & 0x200 != 0;
    let is_duplicate = record.flags & 0x400 != 0;
    let is_supplementary = record.flags & 0x800 != 0;
    let is_read1 = record.flags & 0x40 != 0;
    let is_read2 = record.flags & 0x80 != 0;

    if is_unmapped && record.ref_id >= 0 {
        collector.push(record_finding(
            FindingSeverity::Error,
            "contradictory_mapping_state",
            "Record has unmapped flag set but refID is non-negative.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }
    if !is_unmapped && record.ref_id < 0 {
        collector.push(record_finding(
            FindingSeverity::Error,
            "contradictory_mapping_state",
            "Record has unmapped flag unset but refID is negative.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }
    if !is_unmapped && record.pos < 0 {
        collector.push(record_finding(
            FindingSeverity::Error,
            "negative_position_for_mapped_record",
            "Mapped-looking record has a negative position.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }
    if record.next_ref_id >= 0 && record.next_pos < 0 {
        collector.push(record_finding(
            FindingSeverity::Warning,
            "mate_position_inconsistency",
            "Record has non-negative next_refID but negative next_pos.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }
    if is_proper_pair && !is_paired {
        collector.push(record_finding(
            FindingSeverity::Warning,
            "proper_pair_without_paired_flag",
            "Record sets proper-pair while paired flag is unset.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }
    if is_read1 && is_read2 {
        collector.push(record_finding(
            FindingSeverity::Warning,
            "read1_and_read2_both_set",
            "Record has both read1 and read2 flags set.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }
    if is_secondary && is_supplementary {
        collector.push(record_finding(
            FindingSeverity::Warning,
            "secondary_and_supplementary_both_set",
            "Record has both secondary and supplementary flags set.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }
    if !is_unmapped && record.n_cigar_op == 0 {
        collector.push(record_finding(
            FindingSeverity::Warning,
            "mapped_record_without_cigar",
            "Mapped-looking record has zero CIGAR operations.",
            record_index,
            reference_name.clone(),
            None,
        ));
    }

    let _ = is_qc_fail;
    let _ = is_duplicate;
    let _ = record.mapping_quality;
    let _ = record.l_seq;

    let mut seen_tags = HashSet::new();
    let traversal_result = traverse_aux_fields(&record.aux_bytes, |field| {
        if !seen_tags.insert(field.tag) {
            let tag = String::from_utf8_lossy(&field.tag).into_owned();
            collector.push(record_finding(
                FindingSeverity::Warning,
                "duplicate_aux_tag",
                &format!("Record contains duplicate auxiliary tag {}.", tag),
                record_index,
                reference_name.clone(),
                Some(tag),
            ));
        }
        Ok(())
    });

    if let Err(detail) = traversal_result {
        collector.push(record_finding(
            FindingSeverity::Error,
            "malformed_aux_region",
            &detail,
            record_index,
            reference_name,
            None,
        ));
    }
}

fn record_finding(
    severity: FindingSeverity,
    code: &str,
    message: &str,
    record_index: u64,
    reference_name: Option<String>,
    tag: Option<String>,
) -> ValidationFinding {
    ValidationFinding {
        severity,
        scope: if code.contains("aux") {
            FindingScope::Aux
        } else {
            FindingScope::Record
        },
        code: code.to_string(),
        message: message.to_string(),
        record_index: Some(record_index),
        reference_name,
        tag,
    }
}

fn build_payload(
    mode: ValidationMode,
    valid: bool,
    records_examined: u64,
    full_file_examined: bool,
    collector: FindingCollector,
) -> ValidatePayload {
    ValidatePayload {
        format: "BAM",
        mode,
        valid,
        summary: ValidationSummary {
            header_valid: collector.header_error_count == 0,
            records_examined,
            full_file_examined,
            errors: collector.error_count,
            warnings: collector.warning_count,
            infos: collector.info_count,
        },
        findings: collector.findings,
        semantic_note: match mode {
            ValidationMode::HeaderOnly => "Validation covered BAM file-level and header-level structure only. It does not imply that alignment records are structurally valid.".to_string(),
            ValidationMode::BoundedRecords => "Validation covered BAM structure and selected internal consistency checks for the examined portion of the file only. It does not imply biological correctness or external reference concordance.".to_string(),
            ValidationMode::Full => "Validation covers BAM structure and selected internal consistency checks. It does not imply biological correctness or external reference concordance.".to_string(),
        },
    }
}

fn header_reference_names(header: &HeaderPayload) -> Vec<String> {
    header
        .header
        .references
        .iter()
        .map(|reference| reference.name.clone())
        .collect()
}

fn parse_textual_sq(raw_header_text: &str) -> HashMap<String, Option<u32>> {
    let mut sq = HashMap::new();
    for line in raw_header_text
        .lines()
        .filter(|line| line.starts_with("@SQ\t"))
    {
        let mut name = None;
        let mut length = None;
        for field in line.split('\t').skip(1) {
            if let Some((tag, value)) = field.split_once(':') {
                match tag {
                    "SN" => name = Some(value.to_string()),
                    "LN" => length = value.parse::<u32>().ok(),
                    _ => {}
                }
            }
        }
        if let Some(name) = name {
            sq.insert(name, length);
        }
    }
    sq
}

struct FindingCollector {
    options: ValidationOptions,
    findings: Vec<ValidationFinding>,
    error_count: u64,
    warning_count: u64,
    info_count: u64,
    header_error_count: u64,
}

impl FindingCollector {
    fn new(options: ValidationOptions) -> Self {
        Self {
            options,
            findings: Vec::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            header_error_count: 0,
        }
    }

    fn push(&mut self, finding: ValidationFinding) {
        match finding.severity {
            FindingSeverity::Error => {
                self.error_count += 1;
                if matches!(finding.scope, FindingScope::Header) {
                    self.header_error_count += 1;
                }
                if self
                    .findings
                    .iter()
                    .filter(|finding| finding.severity == FindingSeverity::Error)
                    .count()
                    < self.options.max_errors
                {
                    self.findings.push(finding);
                }
            }
            FindingSeverity::Warning => {
                self.warning_count += 1;
                let warning_count = self
                    .findings
                    .iter()
                    .filter(|finding| finding.severity == FindingSeverity::Warning)
                    .count();
                if self.options.include_warnings && warning_count < self.options.max_warnings {
                    self.findings.push(finding);
                }
            }
            FindingSeverity::Info => {
                self.info_count += 1;
                self.findings.push(finding);
            }
        }
    }

    fn should_stop(&self) -> bool {
        self.options.fail_fast && self.error_count > 0
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::formats::bgzf::test_support::{
        build_bam_file_with_header_and_records, write_temp_file,
    };

    use super::{FindingSeverity, ValidationMode, ValidationOptions, validate_bam};

    struct RecordSpec<'a> {
        ref_id: i32,
        pos: i32,
        flags: u16,
        read_name: &'a str,
        next_ref_id: i32,
        next_pos: i32,
        n_cigar_op: u16,
        l_seq: i32,
        aux: &'a [u8],
    }

    fn build_record(spec: RecordSpec<'_>) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(spec.read_name.as_bytes());
        variable.push(0);
        variable.extend(std::iter::repeat_n(0_u8, usize::from(spec.n_cigar_op) * 4));
        variable.extend(std::iter::repeat_n(
            0_u8,
            usize::try_from(spec.l_seq.max(0)).unwrap_or(0).div_ceil(2),
        ));
        variable.extend(std::iter::repeat_n(
            0_u8,
            usize::try_from(spec.l_seq.max(0)).unwrap_or(0),
        ));
        variable.extend_from_slice(spec.aux);

        let l_read_name = spec.read_name.len() as u32 + 1;
        let bin_mq_nl = l_read_name;
        let flag_nc = ((spec.flags as u32) << 16) | u32::from(spec.n_cigar_op);
        let block_size = 32 + variable.len();

        let mut record = Vec::new();
        record.extend_from_slice(&(block_size as i32).to_le_bytes());
        record.extend_from_slice(&spec.ref_id.to_le_bytes());
        record.extend_from_slice(&spec.pos.to_le_bytes());
        record.extend_from_slice(&bin_mq_nl.to_le_bytes());
        record.extend_from_slice(&flag_nc.to_le_bytes());
        record.extend_from_slice(&spec.l_seq.to_le_bytes());
        record.extend_from_slice(&spec.next_ref_id.to_le_bytes());
        record.extend_from_slice(&spec.next_pos.to_le_bytes());
        record.extend_from_slice(&0_i32.to_le_bytes());
        record.extend_from_slice(&variable);
        record
    }

    #[test]
    fn validates_header_only_mode() {
        let bytes =
            build_bam_file_with_header_and_records("@SQ\tSN:chr1\tLN:10\n", &[("chr1", 10)], &[]);
        let path = write_temp_file("validate-header", "bam", &bytes);
        let payload = validate_bam(
            &path,
            ValidationOptions {
                max_errors: 10,
                max_warnings: 10,
                header_only: true,
                record_limit: None,
                fail_fast: false,
                include_warnings: true,
            },
        )
        .expect("validation should complete");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(payload.valid);
        assert!(matches!(payload.mode, ValidationMode::HeaderOnly));
    }

    #[test]
    fn reports_duplicate_aux_tag_warning() {
        let record = build_record(RecordSpec {
            ref_id: 0,
            pos: 1,
            flags: 0,
            read_name: "read1",
            next_ref_id: -1,
            next_pos: -1,
            n_cigar_op: 0,
            l_seq: 0,
            aux: b"NMi\x01\0\0\0NMi\x02\0\0\0",
        });
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record],
        );
        let path = write_temp_file("validate-dup-aux", "bam", &bytes);
        let payload = validate_bam(
            &path,
            ValidationOptions {
                max_errors: 10,
                max_warnings: 10,
                header_only: false,
                record_limit: Some(10),
                fail_fast: false,
                include_warnings: true,
            },
        )
        .expect("validation should complete");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(
            payload
                .findings
                .iter()
                .any(|finding| finding.code == "duplicate_aux_tag"
                    && finding.severity == FindingSeverity::Warning)
        );
    }

    #[test]
    fn reports_contradictory_mapping_state() {
        let record = build_record(RecordSpec {
            ref_id: 0,
            pos: 1,
            flags: 0x4,
            read_name: "read1",
            next_ref_id: -1,
            next_pos: -1,
            n_cigar_op: 0,
            l_seq: 0,
            aux: b"",
        });
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record],
        );
        let path = write_temp_file("validate-map-state", "bam", &bytes);
        let payload = validate_bam(
            &path,
            ValidationOptions {
                max_errors: 10,
                max_warnings: 10,
                header_only: false,
                record_limit: None,
                fail_fast: false,
                include_warnings: true,
            },
        )
        .expect("validation should complete");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(!payload.valid);
        assert!(
            payload
                .findings
                .iter()
                .any(|finding| finding.code == "contradictory_mapping_state")
        );
    }
}
