//! Provenance-bound JSON records for the AlleleAnchor adapter.

use std::path::PathBuf;

use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    bam::{
        record::BamRecordView,
        records::{decode_bam_qualities, decode_bam_sequence},
        scan::BamScanner,
        tags::{AuxField, traverse_record_aux_fields},
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
};

#[derive(Debug)]
pub struct RecordsRequest {
    pub bam: PathBuf,
    pub input_object_id: String,
    pub reference_assembly: String,
    pub reference_object_id: String,
    pub reference_sha256: String,
    pub reference_fai_sha256: String,
    pub execution_mode: String,
    pub bamana_git_commit: String,
    pub container_image: Option<String>,
    pub container_digest: Option<String>,
    pub include_unmapped: bool,
    pub include_secondary: bool,
    pub include_supplementary: bool,
    pub include_duplicate: bool,
    pub include_qcfail: bool,
    pub min_mapq: u8,
    pub max_records: usize,
}

#[derive(Debug, Serialize)]
pub struct RecordsPayload {
    pub schema_version: &'static str,
    pub operation: &'static str,
    pub ok: bool,
    pub input: InputInfo,
    pub reference: ReferenceInfo,
    pub filters: FilterInfo,
    pub records: Vec<AlignmentRecord>,
    pub rejections: Vec<RejectedRecord>,
    pub provenance: ProvenanceInfo,
    pub error: Option<RecordError>,
}

#[derive(Debug, Serialize)]
pub struct InputInfo {
    pub object_id: String,
    pub format: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ReferenceInfo {
    pub assembly: String,
    pub object_id: String,
    pub fasta_sha256: String,
    pub fai_sha256: String,
    pub source_used: &'static str,
}

#[derive(Debug, Serialize)]
pub struct FilterInfo {
    pub requested: u64,
    pub retained: u64,
    pub rejected: u64,
}

#[derive(Debug, Serialize)]
pub struct ProvenanceInfo {
    pub bamana_version: &'static str,
    pub bamana_git_commit: String,
    pub execution_mode: String,
    pub container_image: Option<String>,
    pub container_digest: Option<String>,
    pub command: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RecordError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AlignmentRecord {
    pub read_id: String,
    pub reference_name: String,
    pub reference_start: u32,
    pub reference_end: u32,
    pub mapping_quality: u8,
    pub flags: u16,
    pub cigar: String,
    pub sequence: String,
    pub qualities: Vec<u8>,
    pub auxiliary_tags: Vec<AuxiliaryTag>,
}

#[derive(Debug, Serialize)]
pub struct RejectedRecord {
    pub read_id: String,
    pub reason: &'static str,
    pub flags: u16,
    pub mapping_quality: u8,
    pub auxiliary_tags: Vec<AuxiliaryTag>,
}

#[derive(Debug, Serialize)]
pub struct AuxiliaryTag {
    pub tag: String,
    pub r#type: String,
    pub value: Value,
    pub raw_payload_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_subtype: Option<String>,
}

pub fn run(request: RecordsRequest) -> CommandResponse<RecordsPayload> {
    match run_impl(&request) {
        Ok(payload) => CommandResponse::success("records", Some(request.bam.as_path()), payload),
        Err(error) => CommandResponse::failure("records", Some(request.bam.as_path()), error),
    }
}

fn run_impl(request: &RecordsRequest) -> Result<RecordsPayload, AppError> {
    validate_request(request)?;
    let probe = probe_path(&request.bam)?;
    if probe.detected_format != DetectedFormat::Bam || probe.container != ContainerKind::Bgzf {
        return Err(AppError::UnsupportedInputForCommand {
            path: request.bam.clone(),
            detail: "records requires a native BGZF BAM; normalize SAM/CRAM with consume first."
                .to_string(),
        });
    }

    let mut scanner = BamScanner::open(&request.bam)?;
    let header = scanner.header().clone();
    let input_path = request.bam.clone();
    let mut records = Vec::new();
    let mut requested = 0_u64;
    let mut rejected = Vec::new();
    while request.max_records == 0 || requested < request.max_records as u64 {
        let Some(record) = scanner.next_record()? else {
            break;
        };
        requested += 1;
        if let Some(reason) = rejection_reason(&record, request) {
            rejected.push(rejected_record(&record, reason, &input_path)?);
            continue;
        }
        records.push(to_alignment_record(&record, &header, &input_path)?);
    }

    Ok(RecordsPayload {
        schema_version: "1.0",
        operation: "aligned_records",
        ok: true,
        input: InputInfo {
            object_id: request.input_object_id.clone(),
            format: "bam",
        },
        reference: ReferenceInfo {
            assembly: request.reference_assembly.clone(),
            object_id: request.reference_object_id.clone(),
            fasta_sha256: request.reference_sha256.clone(),
            fai_sha256: request.reference_fai_sha256.clone(),
            source_used: "explicit_fasta",
        },
        filters: FilterInfo {
            requested,
            retained: records.len() as u64,
            rejected: rejected.len() as u64,
        },
        records,
        rejections: rejected,
        provenance: ProvenanceInfo {
            bamana_version: env!("CARGO_PKG_VERSION"),
            bamana_git_commit: request.bamana_git_commit.clone(),
            execution_mode: request.execution_mode.clone(),
            container_image: request.container_image.clone(),
            container_digest: request.container_digest.clone(),
            command: vec!["bamana".to_string(), "records".to_string()],
        },
        error: None,
    })
}

fn validate_request(request: &RecordsRequest) -> Result<(), AppError> {
    let fields = [
        (request.input_object_id.as_str(), "input_object_id"),
        (request.reference_assembly.as_str(), "reference_assembly"),
        (request.reference_object_id.as_str(), "reference_object_id"),
    ];
    if let Some((_, field)) = fields.into_iter().find(|(value, _)| value.is_empty()) {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: format!("{field} must not be empty."),
        });
    }
    if request.reference_sha256.len() != 64
        || !request
            .reference_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "reference_sha256 must contain exactly 64 hexadecimal characters.".to_string(),
        });
    }
    if request.reference_fai_sha256.len() != 64
        || !request
            .reference_fai_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "reference_fai_sha256 must contain exactly 64 hexadecimal characters."
                .to_string(),
        });
    }
    if request.execution_mode != "host_binary" && request.execution_mode != "container" {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "execution_mode must be host_binary or container.".to_string(),
        });
    }
    if request.bamana_git_commit.len() < 7
        || request.bamana_git_commit.len() > 64
        || !request
            .bamana_git_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "bamana_git_commit must contain 7-64 hexadecimal characters.".to_string(),
        });
    }
    match request.execution_mode.as_str() {
        "host_binary"
            if request.container_image.is_some() || request.container_digest.is_some() =>
        {
            return Err(AppError::InvalidFilterRequest {
                path: request.bam.clone(),
                detail: "host_binary execution must not carry container provenance.".to_string(),
            });
        }
        "container" if request.container_image.is_none() || request.container_digest.is_none() => {
            return Err(AppError::InvalidFilterRequest {
                path: request.bam.clone(),
                detail: "container execution requires image and digest provenance.".to_string(),
            });
        }
        _ => {}
    }
    if let Some(digest) = request.container_digest.as_deref()
        && (digest.len() != 71
            || !digest.starts_with("sha256:")
            || !digest[7..].bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(AppError::InvalidFilterRequest {
            path: request.bam.clone(),
            detail: "container_digest must be sha256:<64 hexadecimal characters>.".to_string(),
        });
    }
    Ok(())
}

fn rejection_reason(record: &BamRecordView<'_>, request: &RecordsRequest) -> Option<&'static str> {
    let flags = record.flag_summary();
    if !request.include_unmapped && flags.is_unmapped {
        return Some("unmapped");
    }
    if !request.include_secondary && flags.is_secondary {
        return Some("non_primary");
    }
    if !request.include_supplementary && flags.is_supplementary {
        return Some("non_primary");
    }
    if !request.include_duplicate && flags.is_duplicate {
        return Some("duplicate");
    }
    if !request.include_qcfail && flags.is_qc_fail {
        return Some("qcfail");
    }
    if !flags.is_unmapped && record.mapping_quality() < request.min_mapq {
        return Some("low_mapq");
    }
    None
}

fn to_alignment_record(
    record: &BamRecordView<'_>,
    header: &crate::bam::header::HeaderPayload,
    input_path: &std::path::Path,
) -> Result<AlignmentRecord, AppError> {
    let reference_name = if record.ref_id() < 0 {
        "*".to_string()
    } else {
        header
            .header
            .references
            .get(record.ref_id() as usize)
            .map(|reference| reference.name.clone())
            .ok_or_else(|| AppError::InvalidRecord {
                path: input_path.to_path_buf(),
                detail: format!(
                    "record reference index {} was not in the BAM header",
                    record.ref_id()
                ),
            })?
    };
    let sequence =
        decode_bam_sequence(record.sequence_bytes(), record.sequence_len()).map_err(|detail| {
            AppError::InvalidRecord {
                path: input_path.to_path_buf(),
                detail,
            }
        })?;
    let quality_text =
        decode_bam_qualities(record.quality_bytes()).map_err(|detail| AppError::InvalidRecord {
            path: input_path.to_path_buf(),
            detail,
        })?;
    if quality_text == "*" {
        return Err(AppError::InvalidRecord {
            path: input_path.to_path_buf(),
            detail: "missing BAM qualities cannot be emitted as per-base evidence.".to_string(),
        });
    }
    let qualities = quality_text
        .bytes()
        .map(|quality| quality.saturating_sub(b'!'))
        .collect();
    let auxiliary_tags = parse_auxiliary_tags(record, input_path)?;
    let reference_start = record.pos().max(0) as u32;
    let reference_end =
        reference_start.saturating_add(reference_span(record.cigar_bytes()).map_err(|detail| {
            AppError::InvalidRecord {
                path: input_path.to_path_buf(),
                detail,
            }
        })?);
    Ok(AlignmentRecord {
        read_id: record.read_name().to_string(),
        reference_name,
        reference_start,
        reference_end,
        mapping_quality: record.mapping_quality(),
        flags: record.flags(),
        cigar: decode_cigar(record.cigar_bytes()).map_err(|detail| AppError::InvalidRecord {
            path: input_path.to_path_buf(),
            detail,
        })?,
        sequence,
        qualities,
        auxiliary_tags,
    })
}

fn rejected_record(
    record: &BamRecordView<'_>,
    reason: &'static str,
    input_path: &std::path::Path,
) -> Result<RejectedRecord, AppError> {
    Ok(RejectedRecord {
        read_id: record.read_name().to_string(),
        reason,
        flags: record.flags(),
        mapping_quality: record.mapping_quality(),
        auxiliary_tags: parse_auxiliary_tags(record, input_path)?,
    })
}

fn parse_auxiliary_tags(
    record: &BamRecordView<'_>,
    input_path: &std::path::Path,
) -> Result<Vec<AuxiliaryTag>, AppError> {
    let mut tags = Vec::new();
    traverse_record_aux_fields(record, |field| {
        tags.push(auxiliary_tag(field)?);
        Ok(())
    })
    .map_err(|detail| AppError::TagParseUncertainty {
        path: input_path.to_path_buf(),
        detail,
    })?;
    Ok(tags)
}

fn auxiliary_tag(field: AuxField<'_>) -> Result<AuxiliaryTag, String> {
    let value = match field.type_code {
        b'A' => json!(
            field
                .payload
                .first()
                .copied()
                .map(char::from)
                .unwrap_or('\0')
        ),
        b'c' => json!(read_signed(field.payload, 1)? as i64),
        b'C' => json!(field.payload.first().copied().unwrap_or_default()),
        b's' => json!(read_signed(field.payload, 2)? as i64),
        b'S' => json!(read_unsigned(field.payload, 2)?),
        b'i' => json!(read_signed(field.payload, 4)? as i64),
        b'I' => json!(read_unsigned(field.payload, 4)?),
        b'f' => json!(f32::from_le_bytes(four_bytes(field.payload)?)),
        b'Z' | b'H' => json!(String::from_utf8_lossy(
            field.payload.strip_suffix(&[0]).unwrap_or(field.payload)
        )),
        b'B' => json!(decode_b_array(field.payload)?),
        other => {
            return Err(format!(
                "unsupported BAM auxiliary type '{}'.",
                other as char
            ));
        }
    };
    let array_subtype = (field.type_code == b'B').then(|| {
        field
            .payload
            .first()
            .copied()
            .map(char::from)
            .unwrap_or('?')
            .to_string()
    });
    Ok(AuxiliaryTag {
        tag: String::from_utf8_lossy(&field.tag).into_owned(),
        r#type: char::from(field.type_code).to_string(),
        value,
        raw_payload_hex: field
            .payload
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        array_subtype,
    })
}

fn read_signed(payload: &[u8], width: usize) -> Result<i64, String> {
    let value = read_unsigned(payload, width)?;
    let bits = width * 8;
    Ok(((value << (64 - bits)) as i64) >> (64 - bits))
}

fn read_unsigned(payload: &[u8], width: usize) -> Result<u64, String> {
    if payload.len() != width {
        return Err(format!(
            "auxiliary integer had {} bytes, expected {width}.",
            payload.len()
        ));
    }
    let mut bytes = [0_u8; 8];
    bytes[..width].copy_from_slice(payload);
    Ok(u64::from_le_bytes(bytes))
}

fn four_bytes(payload: &[u8]) -> Result<[u8; 4], String> {
    payload
        .try_into()
        .map_err(|_| "auxiliary float had an invalid width.".to_string())
}

fn decode_b_array(payload: &[u8]) -> Result<Vec<Value>, String> {
    if payload.len() < 5 {
        return Err("auxiliary B-array was shorter than its subtype and count.".to_string());
    }
    let subtype = payload[0];
    let count = i32::from_le_bytes(payload[1..5].try_into().unwrap());
    if count < 0 {
        return Err("auxiliary B-array count was negative.".to_string());
    }
    let width = match subtype {
        b'c' | b'C' => 1,
        b's' | b'S' => 2,
        b'i' | b'I' | b'f' => 4,
        _ => {
            return Err(format!(
                "unsupported auxiliary B-array subtype '{}'.",
                subtype as char
            ));
        }
    };
    let expected = 5 + count as usize * width;
    if payload.len() != expected {
        return Err("auxiliary B-array payload length did not match its count.".to_string());
    }
    (0..count as usize)
        .map(|index| {
            let start = 5 + index * width;
            let bytes = &payload[start..start + width];
            match subtype {
                b'c' => Ok(json!(read_signed(bytes, 1)?)),
                b'C' => Ok(json!(read_unsigned(bytes, 1)?)),
                b's' => Ok(json!(read_signed(bytes, 2)?)),
                b'S' => Ok(json!(read_unsigned(bytes, 2)?)),
                b'i' => Ok(json!(read_signed(bytes, 4)?)),
                b'I' => Ok(json!(read_unsigned(bytes, 4)?)),
                b'f' => Ok(json!(f32::from_le_bytes(four_bytes(bytes)?))),
                _ => unreachable!(),
            }
        })
        .collect()
}

fn decode_cigar(bytes: &[u8]) -> Result<String, String> {
    let mut output = String::new();
    if bytes.len() % 4 != 0 {
        return Err("CIGAR payload was not a multiple of four bytes.".to_string());
    }
    for chunk in bytes.chunks_exact(4) {
        let encoded = u32::from_le_bytes(chunk.try_into().unwrap());
        let operation = match encoded & 0x0f {
            0 => 'M',
            1 => 'I',
            2 => 'D',
            3 => 'N',
            4 => 'S',
            5 => 'H',
            6 => 'P',
            7 => '=',
            8 => 'X',
            code => return Err(format!("CIGAR operation code {code} was invalid.")),
        };
        output.push_str(&(encoded >> 4).to_string());
        output.push(operation);
    }
    Ok(output)
}

fn reference_span(bytes: &[u8]) -> Result<u32, String> {
    if bytes.len() % 4 != 0 {
        return Err("CIGAR payload was not a multiple of four bytes.".to_string());
    }
    bytes.chunks_exact(4).try_fold(0_u32, |span, chunk| {
        let encoded = u32::from_le_bytes(chunk.try_into().unwrap());
        let consumes_reference = matches!(encoded & 0x0f, 0 | 2 | 3 | 7 | 8);
        if consumes_reference {
            span.checked_add(encoded >> 4)
                .ok_or_else(|| "CIGAR reference span overflowed u32.".to_string())
        } else {
            Ok(span)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{RecordsRequest, decode_cigar, reference_span, run};
    use crate::bgzf::test_support::{build_bam_file_with_header_and_records, write_temp_file};

    fn request(path: std::path::PathBuf, max_records: usize) -> RecordsRequest {
        RecordsRequest {
            bam: path,
            input_object_id: "aaid:v1:object:fixture-bam".to_string(),
            reference_assembly: "CHM13v2.0".to_string(),
            reference_object_id: "aaid:v1:object:fixture-fasta".to_string(),
            reference_sha256: "a".repeat(64),
            reference_fai_sha256: "b".repeat(64),
            execution_mode: "host_binary".to_string(),
            bamana_git_commit: "efd1d117b24a90a067d419383ac233350fed5d9a".to_string(),
            container_image: None,
            container_digest: None,
            include_unmapped: false,
            include_secondary: false,
            include_supplementary: false,
            include_duplicate: false,
            include_qcfail: false,
            min_mapq: 20,
            max_records,
        }
    }

    fn record(name: &str, flags: u16, aux: &[u8]) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(name.as_bytes());
        variable.push(0);
        variable.extend_from_slice(&(8_u32 << 4).to_le_bytes());
        variable.extend_from_slice(&[0x12, 0x48, 0x12, 0x48]);
        variable.extend_from_slice(&[40; 8]);
        variable.extend_from_slice(aux);
        let block_size = 32 + variable.len();
        let mut output = Vec::new();
        output.extend_from_slice(&(block_size as i32).to_le_bytes());
        output.extend_from_slice(&0_i32.to_le_bytes());
        output.extend_from_slice(&0_i32.to_le_bytes());
        output.extend_from_slice(&((60_u32 << 8) | (name.len() as u32 + 1)).to_le_bytes());
        output.extend_from_slice(&(((flags as u32) << 16) | 1).to_le_bytes());
        output.extend_from_slice(&8_i32.to_le_bytes());
        output.extend_from_slice(&(-1_i32).to_le_bytes());
        output.extend_from_slice(&(-1_i32).to_le_bytes());
        output.extend_from_slice(&0_i32.to_le_bytes());
        output.extend_from_slice(&variable);
        output
    }

    #[test]
    fn records_command_filters_duplicate_and_preserves_rejection_reason() {
        let aux = b"NMi\0\0\0\0RGZsynthetic\0";
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr20\tLN:32\tAS:CHM13v2.0\n",
            &[("chr20", 32)],
            &[record("keep", 0, aux), record("drop", 0x400, aux)],
        );
        let path = write_temp_file("records-filter", "bam", &bytes);
        let response = run(request(path.clone(), 0));
        let payload = response.data.expect("successful records payload");
        assert_eq!(payload.records.len(), 1);
        assert_eq!(payload.records[0].read_id, "keep");
        assert_eq!(payload.records[0].reference_end, 8);
        assert_eq!(payload.rejections[0].reason, "duplicate");
        assert_eq!(payload.filters.requested, 2);
        assert_eq!(payload.filters.rejected, 1);
        std::fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn records_command_honours_bounded_scan() {
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr20\tLN:32\n",
            &[("chr20", 32)],
            &[record("first", 0, &[]), record("second", 0, &[])],
        );
        let path = write_temp_file("records-bound", "bam", &bytes);
        let payload = run(request(path.clone(), 1)).data.expect("payload");
        assert_eq!(payload.filters.requested, 1);
        assert_eq!(payload.records[0].read_id, "first");
        std::fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn records_command_fails_before_emitting_malformed_auxiliary_data() {
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr20\tLN:32\n",
            &[("chr20", 32)],
            &[record("broken", 0, b"NMZnot-terminated")],
        );
        let path = write_temp_file("records-malformed-aux", "bam", &bytes);
        let response = run(request(path.clone(), 0));
        assert!(response.data.is_none());
        assert_eq!(
            response.error.expect("structured failure").code,
            "parse_uncertainty"
        );
        std::fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn records_command_rejects_missing_qualities() {
        let mut missing = record("missing", 0, &[]);
        let quality_start = 4 + 32 + "missing".len() + 1 + 4 + 4;
        missing[quality_start..quality_start + 8].fill(0xff);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr20\tLN:32\n",
            &[("chr20", 32)],
            &[missing],
        );
        let path = write_temp_file("records-missing-quality", "bam", &bytes);
        let response = run(request(path.clone(), 0));
        assert!(response.data.is_none());
        assert_eq!(
            response.error.expect("structured failure").code,
            "invalid_record"
        );
        std::fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn cigar_and_reference_span_use_bam_coordinates() {
        let cigar = [
            (8_u32 << 4),
            (2_u32 << 4) | 1,
            (3_u32 << 4) | 2,
            (4_u32 << 4) | 3,
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
        assert_eq!(decode_cigar(&cigar).unwrap(), "8M2I3D4N");
        assert_eq!(reference_span(&cigar).unwrap(), 15);
    }
}
