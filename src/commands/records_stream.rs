//! Atomic bounded-memory NDJSON transport for governed alignment records.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    bam::{
        index::{
            IndexKind, IndexResolution, bam_newer_than_index, parse_bai_index,
            resolve_index_for_bam,
        },
        record::BamRecordView,
        region::normalize_region_strings,
        region_plan::plan_bai_region_chunks,
        region_traversal::visit_planned_region_records,
        scan::BamScanner,
    },
    commands::records::{
        AlignmentRecord, RecordsRegionScope, RecordsRequest, RejectedRecord, rejected_record,
        rejection_reason, to_alignment_record, validate_request,
    },
    error::AppError,
    formats::probe::{ContainerKind, DetectedFormat, probe_path},
    json::CommandResponse,
    output_safety::{finalize_completed_output, remove_stale_temp},
};

#[derive(Debug, Serialize)]
pub struct RecordsStreamPayload {
    schema_version: &'static str,
    operation: &'static str,
    ok: bool,
    input: StreamInput,
    reference: StreamReference,
    filters: StreamFilters,
    region_scope: Option<RecordsRegionScope>,
    stream: StreamInfo,
    rejection_counts: BTreeMap<&'static str, u64>,
    provenance: StreamProvenance,
    error: Option<StreamError>,
}

#[derive(Debug, Serialize)]
struct StreamInput {
    object_id: String,
    format: &'static str,
}

#[derive(Debug, Serialize)]
struct StreamReference {
    assembly: String,
    object_id: String,
    fasta_sha256: String,
    fai_sha256: String,
    source_used: &'static str,
}

#[derive(Debug, Serialize)]
struct StreamFilters {
    requested: u64,
    retained: u64,
    rejected: u64,
}

#[derive(Debug, Serialize)]
struct StreamInfo {
    format: &'static str,
    path: String,
    ordering: &'static str,
    record_kind_field: &'static str,
}

#[derive(Debug, Serialize)]
struct StreamProvenance {
    bamana_version: &'static str,
    bamana_git_commit: String,
    execution_mode: String,
    container_image: Option<String>,
    container_digest: Option<String>,
    command: [&'static str; 2],
}

#[derive(Debug, Serialize)]
struct StreamError;

#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum StreamEntry {
    Record(AlignmentRecord),
    Rejection(RejectedRecord),
}

#[derive(Default)]
struct StreamCounts {
    requested: u64,
    retained: u64,
    rejected: u64,
    rejection_counts: BTreeMap<&'static str, u64>,
}

pub fn run(
    request: RecordsRequest,
    output: PathBuf,
    force: bool,
) -> CommandResponse<RecordsStreamPayload> {
    match run_impl(&request, &output, force) {
        Ok(payload) => CommandResponse::success("records", Some(request.bam.as_path()), payload),
        Err(error) => CommandResponse::failure("records", Some(request.bam.as_path()), error),
    }
}

fn run_impl(
    request: &RecordsRequest,
    output: &Path,
    force: bool,
) -> Result<RecordsStreamPayload, AppError> {
    validate_request(request)?;
    if output.exists() && !force {
        return Err(AppError::OutputExists {
            path: output.to_path_buf(),
        });
    }
    let probe = probe_path(&request.bam)?;
    if probe.detected_format != DetectedFormat::Bam || probe.container != ContainerKind::Bgzf {
        return Err(AppError::UnsupportedInputForCommand {
            path: request.bam.clone(),
            detail: "records --stream-out requires a native BGZF BAM; normalize SAM/CRAM with consume first."
                .to_string(),
        });
    }

    let temp = temporary_output_path(output);
    remove_stale_temp(&temp);
    let result = write_stream(request, &temp);
    let (counts, region_scope) = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&temp);
            return Err(error);
        }
    };
    finalize_completed_output(&temp, output, force)?;

    Ok(RecordsStreamPayload {
        schema_version: "1.0",
        operation: "aligned_record_stream",
        ok: true,
        input: StreamInput {
            object_id: request.input_object_id.clone(),
            format: "bam",
        },
        reference: StreamReference {
            assembly: request.reference_assembly.clone(),
            object_id: request.reference_object_id.clone(),
            fasta_sha256: request.reference_sha256.clone(),
            fai_sha256: request.reference_fai_sha256.clone(),
            source_used: "explicit_fasta",
        },
        filters: StreamFilters {
            requested: counts.requested,
            retained: counts.retained,
            rejected: counts.rejected,
        },
        region_scope,
        stream: StreamInfo {
            format: "application/x-ndjson",
            path: output
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            ordering: "source_virtual_offset_order",
            record_kind_field: "kind",
        },
        rejection_counts: counts.rejection_counts,
        provenance: StreamProvenance {
            bamana_version: env!("CARGO_PKG_VERSION"),
            bamana_git_commit: request.bamana_git_commit.clone(),
            execution_mode: request.execution_mode.clone(),
            container_image: request.container_image.clone(),
            container_digest: request.container_digest.clone(),
            command: ["bamana", "records"],
        },
        error: None,
    })
}

fn write_stream(
    request: &RecordsRequest,
    temp: &Path,
) -> Result<(StreamCounts, Option<RecordsRegionScope>), AppError> {
    let mut scanner = BamScanner::open(&request.bam)?;
    let header = scanner.header().clone();
    let mut writer = BufWriter::new(File::create(temp).map_err(|error| AppError::WriteError {
        path: temp.to_path_buf(),
        message: error.to_string(),
    })?);
    let mut counts = StreamCounts::default();

    let region_scope = if request.regions.is_empty() {
        while request.max_records == 0 || counts.requested < request.max_records as u64 {
            let Some(record) = scanner.next_record()? else {
                break;
            };
            emit_record(
                &record,
                request,
                &header,
                &request.bam,
                temp,
                &mut writer,
                &mut counts,
            )?;
        }
        None
    } else {
        let regions =
            normalize_region_strings(&request.regions, &header.header.references, &request.bam)?;
        let resolved = match resolve_index_for_bam(&request.bam) {
            IndexResolution::Present(index) if index.kind == IndexKind::Bai => index,
            _ => {
                return Err(AppError::MissingIndex {
                    path: request.bam.clone(),
                    detail: Some(
                        "records --region --stream-out requires a usable BAI sidecar; scan fallback is prohibited"
                            .to_string(),
                    ),
                });
            }
        };
        if bam_newer_than_index(&request.bam, &resolved.path) == Some(true) {
            return Err(AppError::MissingIndex {
                path: resolved.path,
                detail: Some("records streaming rejected a stale BAI sidecar".to_string()),
            });
        }
        let index = parse_bai_index(&resolved.path, header.header.references.len())?;
        let plan = plan_bai_region_chunks(&regions, &index, &resolved.path, resolved.kind, false)?;
        let stats = visit_planned_region_records(&request.bam, &regions, &plan, |matched| {
            if request.max_records != 0 && counts.requested >= request.max_records as u64 {
                return Ok(());
            }
            let record = BamRecordView::parse(&matched.raw_record).map_err(|error| {
                AppError::InvalidRecord {
                    path: request.bam.clone(),
                    detail: error.detail().to_string(),
                }
            })?;
            emit_record(
                &record,
                request,
                &header,
                &request.bam,
                temp,
                &mut writer,
                &mut counts,
            )
        })?;
        Some(RecordsRegionScope {
            coordinate_base: regions.coordinate_base,
            interval_semantics: regions.interval_semantics,
            duplicate_policy: "deduplicate_physical_records_by_virtual_offset",
            regions: regions.regions,
            execution: "indexed_stream",
            index_path: resolved.path.to_string_lossy().to_string(),
            chunks_traversed: stats.chunks_traversed,
            raw_records_seen: stats.raw_records_seen,
            duplicate_records_suppressed: stats.duplicate_records_suppressed,
        })
    };
    writer.flush().map_err(|error| AppError::WriteError {
        path: temp.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok((counts, region_scope))
}

fn emit_record(
    record: &BamRecordView<'_>,
    request: &RecordsRequest,
    header: &crate::bam::header::HeaderPayload,
    input_path: &Path,
    output_path: &Path,
    writer: &mut BufWriter<File>,
    counts: &mut StreamCounts,
) -> Result<(), AppError> {
    counts.requested += 1;
    let entry = if let Some(reason) = rejection_reason(record, request) {
        counts.rejected += 1;
        *counts.rejection_counts.entry(reason).or_default() += 1;
        StreamEntry::Rejection(rejected_record(record, reason, input_path)?)
    } else {
        counts.retained += 1;
        StreamEntry::Record(to_alignment_record(record, header, input_path)?)
    };
    serde_json::to_writer(&mut *writer, &entry).map_err(|error| AppError::WriteError {
        path: output_path.to_path_buf(),
        message: error.to_string(),
    })?;
    writer
        .write_all(b"\n")
        .map_err(|error| AppError::WriteError {
            path: output_path.to_path_buf(),
            message: error.to_string(),
        })
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let name = output.file_name().unwrap_or_default().to_string_lossy();
    output.with_file_name(format!(".{name}.bamana-records-{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;

    use super::run;
    use crate::{
        bam::index::{build_bai_index_from_bam, write_bai_index},
        bgzf::test_support::{build_bam_file_with_header_and_records, write_temp_file},
        commands::records::RecordsRequest,
    };

    fn request(path: std::path::PathBuf) -> RecordsRequest {
        RecordsRequest {
            bam: path,
            regions: vec!["chr20:1-20".to_string()],
            input_object_id: "aaid:v1:object:fixture-bam".to_string(),
            reference_assembly: "CHM13v2.0".to_string(),
            reference_object_id: "aaid:v1:object:fixture-fasta".to_string(),
            reference_sha256: "a".repeat(64),
            reference_fai_sha256: "b".repeat(64),
            execution_mode: "host_binary".to_string(),
            bamana_git_commit: "c725454960f380d55a33aa8ae04852b92fec45c4".to_string(),
            container_image: None,
            container_digest: None,
            include_unmapped: false,
            include_secondary: false,
            include_supplementary: false,
            include_duplicate: false,
            include_qcfail: false,
            min_mapq: 0,
            max_records: 0,
        }
    }

    fn record(name: &str, position: i32) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(name.as_bytes());
        variable.push(0);
        variable.extend_from_slice(&(8_u32 << 4).to_le_bytes());
        variable.extend_from_slice(&[0x12, 0x48, 0x12, 0x48]);
        variable.extend_from_slice(&[40; 8]);
        let block_size = 32 + variable.len();
        let mut output = Vec::new();
        output.extend_from_slice(&(block_size as i32).to_le_bytes());
        output.extend_from_slice(&0_i32.to_le_bytes());
        output.extend_from_slice(&position.to_le_bytes());
        output.extend_from_slice(&((60_u32 << 8) | (name.len() as u32 + 1)).to_le_bytes());
        output.extend_from_slice(&1_u32.to_le_bytes());
        output.extend_from_slice(&8_i32.to_le_bytes());
        output.extend_from_slice(&(-1_i32).to_le_bytes());
        output.extend_from_slice(&(-1_i32).to_le_bytes());
        output.extend_from_slice(&0_i32.to_le_bytes());
        output.extend_from_slice(&variable);
        output
    }

    #[test]
    fn indexed_stream_is_atomic_ordered_and_machine_readable() {
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr20\tLN:100\n",
            &[("chr20", 100)],
            &[record("inside", 4), record("outside", 50)],
        );
        let bam = write_temp_file("records-stream", "bam", &bytes);
        let bai = std::path::PathBuf::from(format!("{}.bai", bam.to_string_lossy()));
        write_bai_index(&bai, &build_bai_index_from_bam(&bam).unwrap()).unwrap();
        let output = bam.with_extension("records.ndjson");

        let response = run(request(bam.clone()), output.clone(), false);
        assert!(response.ok, "stream failed: {:?}", response.error);
        let payload = response.data.expect("stream should succeed");
        assert_eq!(payload.filters.requested, 1);
        assert_eq!(payload.filters.retained, 1);
        assert_eq!(payload.region_scope.unwrap().execution, "indexed_stream");
        let lines = fs::read_to_string(&output).unwrap();
        let rows = lines
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["kind"], "record");
        assert_eq!(rows[0]["data"]["read_id"], "inside");
        assert!(
            !output
                .with_file_name(format!(
                    ".{}.bamana-records-{}.tmp",
                    output.file_name().unwrap().to_string_lossy(),
                    std::process::id()
                ))
                .exists()
        );

        fs::remove_file(output).unwrap();
        fs::remove_file(bai).unwrap();
        fs::remove_file(bam).unwrap();
    }
}
