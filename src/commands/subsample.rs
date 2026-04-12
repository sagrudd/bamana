use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::{
    bam::{
        header::{parse_bam_header_from_reader, serialize_bam_header_payload},
        index::{IndexKind, IndexResolution, resolve_index_for_bam},
        reader::BamReader,
        records::{BAM_FSECONDARY, BAM_FSUPPLEMENTARY, RecordLayout, read_next_record_layout},
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
    fastq::{FastqRecord, FastqWriter, open_fastq_reader, read_next_fastq_record},
    formats::probe::{DetectedFormat, probe_path},
    json::CommandResponse,
    sampling::{
        DeterministicIdentity, SubsampleMode,
        hash::{SplitMix64, fnv1a64, should_keep_fraction},
    },
};

#[derive(Debug)]
pub struct SubsampleRequest {
    pub input: PathBuf,
    pub out: PathBuf,
    pub fraction: f64,
    pub mode: SubsampleMode,
    pub seed: Option<u64>,
    pub identity: DeterministicIdentity,
    pub dry_run: bool,
    pub create_index: bool,
    pub mapped_only: bool,
    pub primary_only: bool,
    pub threads: usize,
    pub force: bool,
}

#[derive(Debug, Clone)]
struct SubsampleConfig {
    input: PathBuf,
    out: PathBuf,
    fraction: f64,
    mode: SubsampleMode,
    seed: Option<u64>,
    identity: DeterministicIdentity,
    dry_run: bool,
    create_index: bool,
    mapped_only: bool,
    primary_only: bool,
    threads: usize,
    force: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubsamplePayload {
    pub format: DetectedFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<SubsampleSelectionPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<SubsampleExecutionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<SubsampleOutputInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<SubsampleIndexInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<SubsampleFilterInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubsampleSelectionPolicy {
    pub mode: SubsampleMode,
    pub fraction_requested: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deterministic_identity: Option<DeterministicIdentity>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubsampleExecutionInfo {
    pub dry_run: bool,
    pub records_examined: u64,
    pub records_retained: u64,
    pub fraction_retained_observed: f64,
    pub order_preserved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligible_records_examined: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubsampleOutputInfo {
    pub path: String,
    pub written: bool,
    pub overwritten: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubsampleIndexInfo {
    pub present_before: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_after: Option<bool>,
    pub reindex_requested: bool,
    pub reindexed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<IndexKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubsampleFilterInfo {
    pub mapped_only: bool,
    pub primary_only: bool,
}

#[derive(Debug)]
struct SubsampleFailure {
    payload: SubsamplePayload,
    error: AppError,
}

pub fn run(request: SubsampleRequest) -> CommandResponse<SubsamplePayload> {
    let config = SubsampleConfig {
        input: request.input.clone(),
        out: request.out,
        fraction: request.fraction,
        mode: request.mode,
        seed: request.seed,
        identity: request.identity,
        dry_run: request.dry_run,
        create_index: request.create_index,
        mapped_only: request.mapped_only,
        primary_only: request.primary_only,
        threads: request.threads.max(1),
        force: request.force,
    };

    match execute(&config) {
        Ok(payload) => {
            CommandResponse::success("subsample", Some(request.input.as_path()), payload)
        }
        Err(SubsampleFailure { payload, error }) => CommandResponse::failure_with_data(
            "subsample",
            Some(request.input.as_path()),
            Some(payload),
            error,
        ),
    }
}

fn execute(config: &SubsampleConfig) -> Result<SubsamplePayload, SubsampleFailure> {
    let detected_format = match probe_path(&config.input) {
        Ok(probe) => probe.detected_format,
        Err(error) => {
            return Err(SubsampleFailure {
                payload: base_payload(DetectedFormat::Unknown, config),
                error,
            });
        }
    };

    validate_request(config, detected_format).map_err(|error| SubsampleFailure {
        payload: base_payload(detected_format, config),
        error,
    })?;

    let resolved_seed = match config.mode {
        SubsampleMode::Random => Some(config.seed.unwrap_or_else(generate_seed)),
        SubsampleMode::Deterministic => None,
    };

    let payload = match detected_format {
        DetectedFormat::Bam => execute_bam(config, resolved_seed),
        DetectedFormat::Fastq | DetectedFormat::FastqGz => {
            execute_fastq(config, detected_format, resolved_seed)
        }
        other => Err(SubsampleFailure {
            payload: base_payload(other, config),
            error: AppError::UnsupportedFormat {
                path: config.input.clone(),
                format: other.to_string(),
            },
        }),
    }?;

    Ok(payload)
}

fn validate_request(
    config: &SubsampleConfig,
    detected_format: DetectedFormat,
) -> Result<(), AppError> {
    if !(config.fraction > 0.0 && config.fraction <= 1.0) {
        return Err(AppError::InvalidFraction {
            path: config.input.clone(),
            detail: "Fraction must be greater than 0 and less than or equal to 1.".to_string(),
        });
    }

    if output_matches_input(&config.input, &config.out) {
        return Err(AppError::InvalidSubsampleMode {
            path: config.input.clone(),
            detail: "Output path must differ from the input path for subsample.".to_string(),
        });
    }

    if matches!(
        detected_format,
        DetectedFormat::Fastq | DetectedFormat::FastqGz
    ) && (config.mapped_only || config.primary_only || config.create_index)
    {
        return Err(AppError::UnsupportedInputForCommand {
            path: config.input.clone(),
            detail:
                "FASTQ and FASTQ.GZ subsampling do not support BAM-only filters or index creation."
                    .to_string(),
        });
    }

    Ok(())
}

fn execute_bam(
    config: &SubsampleConfig,
    resolved_seed: Option<u64>,
) -> Result<SubsamplePayload, SubsampleFailure> {
    let mut reader = BamReader::open(&config.input).map_err(|error| SubsampleFailure {
        payload: base_payload(DetectedFormat::Bam, config),
        error,
    })?;
    let header = parse_bam_header_from_reader(&mut reader).map_err(|error| SubsampleFailure {
        payload: base_payload(DetectedFormat::Bam, config),
        error: map_parse_error(error, &config.input),
    })?;
    let header_payload =
        serialize_bam_header_payload(&header.header.raw_header_text, &header.header.references);

    let index_before = resolve_index_for_bam(&config.input);
    let index_info = build_index_info(&index_before, config, !config.dry_run);
    let output_overwritten = config.out.exists();
    if output_overwritten && !config.force && !config.dry_run {
        return Err(SubsampleFailure {
            payload: partial_payload(
                DetectedFormat::Bam,
                config,
                resolved_seed,
                Some(index_info.clone()),
            ),
            error: AppError::OutputExists {
                path: config.out.clone(),
            },
        });
    }

    let temp_path = temporary_output_path(&config.out);
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    let mut writer = if config.dry_run {
        None
    } else {
        Some(
            BgzfWriter::create(&temp_path).map_err(|error| SubsampleFailure {
                payload: partial_payload(
                    DetectedFormat::Bam,
                    config,
                    resolved_seed,
                    Some(index_info.clone()),
                ),
                error,
            })?,
        )
    };

    if let Some(writer) = &mut writer {
        writer
            .write_all(&header_payload)
            .map_err(|error| SubsampleFailure {
                payload: partial_payload(
                    DetectedFormat::Bam,
                    config,
                    resolved_seed,
                    Some(index_info.clone()),
                ),
                error,
            })?;
    }

    let mut records_examined = 0_u64;
    let mut eligible_records_examined = 0_u64;
    let mut records_retained = 0_u64;
    let mut random = resolved_seed.map(SplitMix64::new);

    loop {
        let next = read_next_record_layout(&mut reader).map_err(|error| SubsampleFailure {
            payload: partial_payload(
                DetectedFormat::Bam,
                config,
                resolved_seed,
                Some(index_info.clone()),
            ),
            error: map_parse_error(error, &config.input),
        })?;
        let Some(record) = next else {
            break;
        };
        records_examined += 1;

        if !bam_record_is_eligible(&record, config) {
            continue;
        }
        eligible_records_examined += 1;

        if should_keep_record_bam(&record, config, &mut random) {
            if let Some(writer) = &mut writer {
                writer
                    .write_all(&serialize_record_layout(&record))
                    .map_err(|error| SubsampleFailure {
                        payload: partial_payload(
                            DetectedFormat::Bam,
                            config,
                            resolved_seed,
                            Some(index_info.clone()),
                        ),
                        error,
                    })?;
            }
            records_retained += 1;
        }
    }

    if let Some(writer) = writer {
        writer.finish().map_err(|error| SubsampleFailure {
            payload: partial_payload(
                DetectedFormat::Bam,
                config,
                resolved_seed,
                Some(index_info.clone()),
            ),
            error,
        })?;
        finalize_output(&temp_path, &config.out, output_overwritten, config.force).map_err(
            |error| SubsampleFailure {
                payload: partial_payload(
                    DetectedFormat::Bam,
                    config,
                    resolved_seed,
                    Some(index_info.clone()),
                ),
                error,
            },
        )?;
    }

    let mut notes = build_notes(config, DetectedFormat::Bam, resolved_seed);
    if config.create_index {
        notes.push(
            "BAM index creation was requested, but index writing remains deferred in this slice; any pre-existing index should be treated as invalid for the subsampled output."
                .to_string(),
        );
    }

    if config.threads > 1 {
        notes.push(format!(
            "Thread count was set to {}, but this first subsample slice currently streams records on a single thread.",
            config.threads
        ));
    }

    Ok(SubsamplePayload {
        format: DetectedFormat::Bam,
        selection: Some(selection_policy(config, resolved_seed)),
        execution: Some(SubsampleExecutionInfo {
            dry_run: config.dry_run,
            records_examined,
            records_retained,
            fraction_retained_observed: observed_fraction(records_retained, records_examined),
            order_preserved: true,
            eligible_records_examined: Some(eligible_records_examined),
        }),
        output: Some(SubsampleOutputInfo {
            path: config.out.to_string_lossy().into_owned(),
            written: !config.dry_run,
            overwritten: output_overwritten && config.force && !config.dry_run,
        }),
        index: Some(index_info),
        filters: Some(SubsampleFilterInfo {
            mapped_only: config.mapped_only,
            primary_only: config.primary_only,
        }),
        notes: Some(notes),
    })
}

fn execute_fastq(
    config: &SubsampleConfig,
    format: DetectedFormat,
    resolved_seed: Option<u64>,
) -> Result<SubsamplePayload, SubsampleFailure> {
    let mut reader = open_fastq_reader(&config.input).map_err(|error| SubsampleFailure {
        payload: base_payload(format, config),
        error,
    })?;

    let output_overwritten = config.out.exists();
    if output_overwritten && !config.force && !config.dry_run {
        return Err(SubsampleFailure {
            payload: partial_payload(format, config, resolved_seed, None),
            error: AppError::OutputExists {
                path: config.out.clone(),
            },
        });
    }

    let temp_path = temporary_output_path(&config.out);
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    let mut writer = if config.dry_run {
        None
    } else {
        Some(
            FastqWriter::create(&temp_path).map_err(|error| SubsampleFailure {
                payload: partial_payload(format, config, resolved_seed, None),
                error,
            })?,
        )
    };

    let mut records_examined = 0_u64;
    let mut records_retained = 0_u64;
    let mut random = resolved_seed.map(SplitMix64::new);

    loop {
        let next = read_next_fastq_record(&mut reader, &config.input).map_err(|error| {
            SubsampleFailure {
                payload: partial_payload(format, config, resolved_seed, None),
                error: map_parse_error(error, &config.input),
            }
        })?;
        let Some(record) = next else {
            break;
        };
        records_examined += 1;

        if should_keep_record_fastq(&record, config, &mut random) {
            if let Some(writer) = &mut writer {
                writer
                    .write_record(&record)
                    .map_err(|error| SubsampleFailure {
                        payload: partial_payload(format, config, resolved_seed, None),
                        error,
                    })?;
            }
            records_retained += 1;
        }
    }

    if let Some(writer) = writer {
        writer.finish().map_err(|error| SubsampleFailure {
            payload: partial_payload(format, config, resolved_seed, None),
            error,
        })?;
        finalize_output(&temp_path, &config.out, output_overwritten, config.force).map_err(
            |error| SubsampleFailure {
                payload: partial_payload(format, config, resolved_seed, None),
                error,
            },
        )?;
    }

    let mut notes = build_notes(config, format, resolved_seed);
    notes.push(
        "FASTQ output compression is inferred from the output filename extension; .gz writes gzip-compressed FASTQ."
            .to_string(),
    );

    Ok(SubsamplePayload {
        format,
        selection: Some(selection_policy(config, resolved_seed)),
        execution: Some(SubsampleExecutionInfo {
            dry_run: config.dry_run,
            records_examined,
            records_retained,
            fraction_retained_observed: observed_fraction(records_retained, records_examined),
            order_preserved: true,
            eligible_records_examined: None,
        }),
        output: Some(SubsampleOutputInfo {
            path: config.out.to_string_lossy().into_owned(),
            written: !config.dry_run,
            overwritten: output_overwritten && config.force && !config.dry_run,
        }),
        index: None,
        filters: None,
        notes: Some(notes),
    })
}

fn selection_policy(
    config: &SubsampleConfig,
    resolved_seed: Option<u64>,
) -> SubsampleSelectionPolicy {
    SubsampleSelectionPolicy {
        mode: config.mode,
        fraction_requested: config.fraction,
        seed: resolved_seed,
        deterministic_identity: match config.mode {
            SubsampleMode::Random => None,
            SubsampleMode::Deterministic => Some(config.identity),
        },
    }
}

fn partial_payload(
    format: DetectedFormat,
    config: &SubsampleConfig,
    resolved_seed: Option<u64>,
    index: Option<SubsampleIndexInfo>,
) -> SubsamplePayload {
    SubsamplePayload {
        format,
        selection: Some(selection_policy(config, resolved_seed)),
        execution: None,
        output: Some(SubsampleOutputInfo {
            path: config.out.to_string_lossy().into_owned(),
            written: false,
            overwritten: false,
        }),
        index,
        filters: if format == DetectedFormat::Bam {
            Some(SubsampleFilterInfo {
                mapped_only: config.mapped_only,
                primary_only: config.primary_only,
            })
        } else {
            None
        },
        notes: None,
    }
}

fn base_payload(format: DetectedFormat, config: &SubsampleConfig) -> SubsamplePayload {
    SubsamplePayload {
        format,
        selection: None,
        execution: None,
        output: Some(SubsampleOutputInfo {
            path: config.out.to_string_lossy().into_owned(),
            written: false,
            overwritten: false,
        }),
        index: None,
        filters: None,
        notes: None,
    }
}

fn build_index_info(
    resolution: &IndexResolution,
    config: &SubsampleConfig,
    writing_output: bool,
) -> SubsampleIndexInfo {
    let (present_before, kind) = match resolution {
        IndexResolution::Present(index) | IndexResolution::Unsupported(index) => {
            (true, Some(index.kind))
        }
        IndexResolution::NotFound => (false, None),
    };

    SubsampleIndexInfo {
        present_before,
        valid_after: if writing_output { Some(false) } else { None },
        reindex_requested: config.create_index,
        reindexed: false,
        kind,
    }
}

fn bam_record_is_eligible(record: &RecordLayout, config: &SubsampleConfig) -> bool {
    if config.mapped_only && (record.flags & 0x4 != 0 || record.ref_id < 0) {
        return false;
    }
    if config.primary_only
        && (record.flags & BAM_FSECONDARY != 0 || record.flags & BAM_FSUPPLEMENTARY != 0)
    {
        return false;
    }
    true
}

fn should_keep_record_bam(
    record: &RecordLayout,
    config: &SubsampleConfig,
    random: &mut Option<SplitMix64>,
) -> bool {
    match config.mode {
        SubsampleMode::Random => {
            let sample = random
                .as_mut()
                .expect("random mode should resolve a seed")
                .next_u64();
            should_keep_fraction(sample, config.fraction)
        }
        SubsampleMode::Deterministic => {
            let identity = build_bam_identity_bytes(record, config.identity);
            should_keep_fraction(fnv1a64(&identity), config.fraction)
        }
    }
}

fn should_keep_record_fastq(
    record: &FastqRecord,
    config: &SubsampleConfig,
    random: &mut Option<SplitMix64>,
) -> bool {
    match config.mode {
        SubsampleMode::Random => {
            let sample = random
                .as_mut()
                .expect("random mode should resolve a seed")
                .next_u64();
            should_keep_fraction(sample, config.fraction)
        }
        SubsampleMode::Deterministic => {
            let identity = build_fastq_identity_bytes(record, config.identity);
            should_keep_fraction(fnv1a64(&identity), config.fraction)
        }
    }
}

fn build_bam_identity_bytes(record: &RecordLayout, identity: DeterministicIdentity) -> Vec<u8> {
    match identity {
        DeterministicIdentity::Qname => record.read_name.as_bytes().to_vec(),
        DeterministicIdentity::QnameSeq => {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(record.read_name.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&(record.l_seq as u64).to_le_bytes());
            bytes.extend_from_slice(&record.sequence_bytes);
            bytes
        }
        DeterministicIdentity::FullRecord => serialize_record_layout(record),
    }
}

fn build_fastq_identity_bytes(record: &FastqRecord, identity: DeterministicIdentity) -> Vec<u8> {
    match identity {
        DeterministicIdentity::Qname => record.read_name.as_bytes().to_vec(),
        DeterministicIdentity::QnameSeq => {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(record.read_name.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(record.sequence.as_bytes());
            bytes
        }
        DeterministicIdentity::FullRecord => {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(record.raw_header_line.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(record.sequence.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(record.plus_line.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(record.quality.as_bytes());
            bytes
        }
    }
}

fn observed_fraction(records_retained: u64, records_examined: u64) -> f64 {
    if records_examined == 0 {
        0.0
    } else {
        records_retained as f64 / records_examined as f64
    }
}

fn build_notes(
    config: &SubsampleConfig,
    format: DetectedFormat,
    resolved_seed: Option<u64>,
) -> Vec<String> {
    let mut notes = vec![
        "Subsample is intended for production workflows and reproducible benchmarking on large BAM and FASTQ inputs."
            .to_string(),
        "Retained record order follows input encounter order.".to_string(),
    ];

    match config.mode {
        SubsampleMode::Random => {
            notes.push(format!(
                "Random seeded subsampling was applied per eligible record with seed {}.",
                resolved_seed.expect("random mode should resolve a seed")
            ));
        }
        SubsampleMode::Deterministic => {
            notes.push(format!(
                "Deterministic hash-based subsampling was applied using the {} identity basis.",
                deterministic_identity_label(config.identity)
            ));
        }
    }

    if config.mapped_only || config.primary_only {
        notes.push(
            "BAM filters are applied before sampling; records excluded by filters are not preserved in the output."
                .to_string(),
        );
    }

    if config.mode == SubsampleMode::Deterministic && config.seed.is_some() {
        notes.push(
            "Seed was supplied, but deterministic mode does not use runtime RNG and therefore reports seed as null."
                .to_string(),
        );
    }

    if config.dry_run {
        notes.push("Dry run only. No output file was written.".to_string());
    }

    if format == DetectedFormat::Bam {
        notes.push(
            "Any pre-existing BAM index for the input must be treated as invalid for the subsampled output unless index regeneration is reported explicitly."
                .to_string(),
        );
    }

    notes
}

fn deterministic_identity_label(identity: DeterministicIdentity) -> &'static str {
    match identity {
        DeterministicIdentity::Qname => "qname",
        DeterministicIdentity::QnameSeq => "qname_seq",
        DeterministicIdentity::FullRecord => "full_record",
    }
}

fn finalize_output(
    temp_path: &Path,
    output_path: &Path,
    output_overwritten: bool,
    force: bool,
) -> Result<(), AppError> {
    if output_overwritten && force {
        fs::remove_file(output_path).map_err(|error| AppError::WriteError {
            path: output_path.to_path_buf(),
            message: error.to_string(),
        })?;
    }

    fs::rename(temp_path, output_path).map_err(|error| AppError::WriteError {
        path: output_path.to_path_buf(),
        message: error.to_string(),
    })
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-subsample-output");
    let suffix = if file_name.ends_with(".gz") {
        ".gz"
    } else if file_name.ends_with(".bam") {
        ".bam"
    } else if file_name.ends_with(".fastq") {
        ".fastq"
    } else {
        ""
    };
    output.with_file_name(format!(
        ".{file_name}.bamana-subsample-{}.tmp{}",
        std::process::id(),
        suffix
    ))
}

fn output_matches_input(input: &Path, output: &Path) -> bool {
    if input == output {
        return true;
    }

    let input_canonical = fs::canonicalize(input).ok();
    let output_canonical = fs::canonicalize(output).ok();
    input_canonical.is_some() && input_canonical == output_canonical
}

fn generate_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ ((std::process::id() as u64) << 32)
}

fn map_parse_error(error: AppError, input_path: &Path) -> AppError {
    match error {
        AppError::InvalidRecord { detail, .. }
        | AppError::InvalidFastq { detail, .. }
        | AppError::TruncatedFile { detail, .. } => AppError::ParseUncertainty {
            path: input_path.to_path_buf(),
            detail,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use flate2::{Compression, write::GzEncoder};

    use super::{SubsampleRequest, run};
    use crate::{
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
        json::CommandResponse,
        sampling::{DeterministicIdentity, SubsampleMode},
    };

    fn read_response_body(response: &CommandResponse<super::SubsamplePayload>) -> String {
        serde_json::to_string(response).expect("response should serialize")
    }

    #[test]
    fn deterministic_fastq_subsampling_is_repeatable() {
        let input = std::env::temp_dir().join(format!(
            "bamana-subsample-deterministic-{}.fastq",
            std::process::id()
        ));
        fs::write(
            &input,
            "@r1\nAAAA\n+\n!!!!\n@r2\nCCCC\n+\n####\n@r3\nGGGG\n+\n$$$$\n",
        )
        .expect("fastq should write");
        let out_a = std::env::temp_dir().join(format!(
            "bamana-subsample-deterministic-a-{}.fastq",
            std::process::id()
        ));
        let out_b = std::env::temp_dir().join(format!(
            "bamana-subsample-deterministic-b-{}.fastq",
            std::process::id()
        ));

        let response_a = run(SubsampleRequest {
            input: input.clone(),
            out: out_a.clone(),
            fraction: 0.5,
            mode: SubsampleMode::Deterministic,
            seed: None,
            identity: DeterministicIdentity::FullRecord,
            dry_run: false,
            create_index: false,
            mapped_only: false,
            primary_only: false,
            threads: 1,
            force: true,
        });
        let response_b = run(SubsampleRequest {
            input: input.clone(),
            out: out_b.clone(),
            fraction: 0.5,
            mode: SubsampleMode::Deterministic,
            seed: None,
            identity: DeterministicIdentity::FullRecord,
            dry_run: false,
            create_index: false,
            mapped_only: false,
            primary_only: false,
            threads: 1,
            force: true,
        });

        assert!(response_a.ok);
        assert!(response_b.ok);
        assert_eq!(
            fs::read_to_string(&out_a).expect("first output should read"),
            fs::read_to_string(&out_b).expect("second output should read")
        );

        fs::remove_file(input).expect("input should be removable");
        fs::remove_file(out_a).expect("first output should be removable");
        fs::remove_file(out_b).expect("second output should be removable");
    }

    #[test]
    fn seeded_random_fastq_subsampling_is_repeatable() {
        let input = std::env::temp_dir().join(format!(
            "bamana-subsample-random-{}.fastq.gz",
            std::process::id()
        ));
        let file = fs::File::create(&input).expect("gzip input should create");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(b"@r1\nAAAA\n+\n!!!!\n@r2\nCCCC\n+\n####\n@r3\nGGGG\n+\n$$$$\n")
            .expect("gzip input should write");
        encoder.finish().expect("gzip input should finish");
        let out_a = std::env::temp_dir().join(format!(
            "bamana-subsample-random-a-{}.fastq.gz",
            std::process::id()
        ));
        let out_b = std::env::temp_dir().join(format!(
            "bamana-subsample-random-b-{}.fastq.gz",
            std::process::id()
        ));

        let response_a = run(SubsampleRequest {
            input: input.clone(),
            out: out_a.clone(),
            fraction: 0.5,
            mode: SubsampleMode::Random,
            seed: Some(12345),
            identity: DeterministicIdentity::FullRecord,
            dry_run: false,
            create_index: false,
            mapped_only: false,
            primary_only: false,
            threads: 1,
            force: true,
        });
        let response_b = run(SubsampleRequest {
            input: input.clone(),
            out: out_b.clone(),
            fraction: 0.5,
            mode: SubsampleMode::Random,
            seed: Some(12345),
            identity: DeterministicIdentity::FullRecord,
            dry_run: false,
            create_index: false,
            mapped_only: false,
            primary_only: false,
            threads: 1,
            force: true,
        });

        assert!(response_a.ok);
        assert!(response_b.ok);
        assert_eq!(
            fs::read(&out_a).expect("first output should read"),
            fs::read(&out_b).expect("second output should read")
        );

        fs::remove_file(input).expect("input should be removable");
        fs::remove_file(out_a).expect("first output should be removable");
        fs::remove_file(out_b).expect("second output should be removable");
    }

    #[test]
    fn invalid_fraction_returns_failure_payload() {
        let input = std::env::temp_dir().join(format!(
            "bamana-subsample-invalid-{}.fastq",
            std::process::id()
        ));
        fs::write(&input, "@r1\nAAAA\n+\n!!!!\n").expect("fastq should write");
        let out = std::env::temp_dir().join(format!(
            "bamana-subsample-invalid-out-{}.fastq",
            std::process::id()
        ));

        let response = run(SubsampleRequest {
            input: input.clone(),
            out: out.clone(),
            fraction: 0.0,
            mode: SubsampleMode::Random,
            seed: Some(1),
            identity: DeterministicIdentity::FullRecord,
            dry_run: true,
            create_index: false,
            mapped_only: false,
            primary_only: false,
            threads: 1,
            force: false,
        });

        assert!(!response.ok);
        let body = read_response_body(&response);
        assert!(body.contains("\"invalid_fraction\""));

        fs::remove_file(input).expect("input should be removable");
    }

    #[test]
    fn bam_subsampling_writes_preserved_header_stream() {
        let input = write_temp_file(
            "subsample-bam-in",
            "bam",
            &build_bam_file_with_header_and_records(
                "@SQ\tSN:chr1\tLN:10\n",
                &[("chr1", 10)],
                &[
                    build_light_record(0, 1, "read1", 0),
                    build_light_record(0, 2, "read2", 0),
                    build_light_record(-1, -1, "read3", 4),
                ],
            ),
        );
        let output = std::env::temp_dir().join(format!(
            "bamana-subsample-bam-out-{}.bam",
            std::process::id()
        ));

        let response = run(SubsampleRequest {
            input: input.clone(),
            out: output.clone(),
            fraction: 1.0,
            mode: SubsampleMode::Deterministic,
            seed: None,
            identity: DeterministicIdentity::Qname,
            dry_run: false,
            create_index: false,
            mapped_only: false,
            primary_only: false,
            threads: 1,
            force: true,
        });

        assert!(response.ok);
        let header =
            crate::bam::header::parse_bam_header(&output).expect("output bam should parse");
        assert_eq!(header.header.references.len(), 1);

        fs::remove_file(input).expect("input should be removable");
        fs::remove_file(output).expect("output should be removable");
    }
}
