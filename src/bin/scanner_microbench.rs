use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use bamana::{
    bam::{
        header::{ReferenceHeaderFields, ReferenceRecord, serialize_bam_header_payload},
        records::{encode_bam_qualities, encode_bam_sequence},
        scan::BamScanner,
        tags::{AuxTypeCode, TagQuery, record_aux_contains_tag},
    },
    bgzf::BgzfWriter,
};
use clap::{Parser, ValueEnum};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "scanner_microbench",
    about = "Run Bamana-native BAM scanner microbenchmarks and emit JSON results."
)]
struct Args {
    #[arg(long = "profile", value_enum, default_value_t = Profile::Small)]
    profile: Profile,
    #[arg(long = "iterations", default_value_t = 5)]
    iterations: usize,
    #[arg(long = "workdir")]
    workdir: Option<PathBuf>,
    #[arg(long = "out")]
    out: Option<PathBuf>,
    #[arg(long = "bamana-bin")]
    bamana_bin: Option<PathBuf>,
    #[arg(long = "keep-fixtures")]
    keep_fixtures: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum Profile {
    Small,
    Medium,
    Large,
}

impl Profile {
    fn record_count(self) -> usize {
        match self {
            Self::Small => 1_024,
            Self::Medium => 50_000,
            Self::Large => 250_000,
        }
    }

    fn reference_count(self) -> usize {
        match self {
            Self::Small => 2,
            Self::Medium => 8,
            Self::Large => 24,
        }
    }

    fn sequence_len(self) -> usize {
        match self {
            Self::Small => 75,
            Self::Medium => 100,
            Self::Large => 150,
        }
    }
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    benchmark: &'static str,
    version: u32,
    profile: Profile,
    iterations: usize,
    fixture: FixtureReport,
    results: BenchmarkResults,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FixtureReport {
    path: String,
    records: usize,
    references: usize,
    sequence_len: usize,
    serialized_payload_bytes: usize,
    bgzf_file_bytes: u64,
    kept: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkResults {
    record_scan_throughput: Measurement,
    selective_field_extraction_throughput: Measurement,
    command_timings: Vec<CommandTiming>,
}

#[derive(Debug, Serialize)]
struct Measurement {
    operation: &'static str,
    iterations: usize,
    records_per_iteration: u64,
    bytes_per_iteration: u64,
    total_seconds: f64,
    min_seconds: f64,
    mean_seconds: f64,
    max_seconds: f64,
    records_per_second: f64,
    bytes_per_second: f64,
    checksum: u64,
}

#[derive(Debug, Serialize)]
struct CommandTiming {
    command: &'static str,
    iterations: usize,
    ok_count: usize,
    total_seconds: f64,
    min_seconds: f64,
    mean_seconds: f64,
    max_seconds: f64,
    exit_codes: Vec<Option<i32>>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.iterations == 0 {
        return Err("--iterations must be greater than zero".into());
    }

    let workdir = args.workdir.unwrap_or_else(default_workdir);
    fs::create_dir_all(&workdir)?;

    let fixture_model = build_fixture_model(args.profile)?;
    let header_payload = serialize_bam_header_payload(
        &workdir.join("scanner-microbench.bam"),
        &fixture_model.header_text,
        &fixture_model.references,
    )?;
    let mut bam_payload = Vec::with_capacity(header_payload.len() + fixture_model.record_bytes);
    bam_payload.extend_from_slice(&header_payload);
    for record in &fixture_model.records {
        bam_payload.extend_from_slice(record);
    }

    let fixture = workdir.join(format!("scanner-microbench-{:?}.bam", args.profile).to_lowercase());
    write_bgzf_payload(&fixture, &bam_payload)?;
    let bgzf_file_bytes = fs::metadata(&fixture)?.len();

    let record_scan_throughput = measure_record_scan(&fixture, bgzf_file_bytes, args.iterations)?;
    let selective_field_extraction_throughput =
        measure_selective_extraction(&fixture, bgzf_file_bytes, args.iterations)?;
    let command_timings = match args.bamana_bin.as_deref() {
        Some(bamana_bin) => {
            let subsample_output = workdir.join("scanner-microbench-subsample-out.bam");
            let sort_output = workdir.join("scanner-microbench-sort-out.bam");
            let deduplicate_output = workdir.join("scanner-microbench-deduplicate-out.bam");
            vec![
                measure_command(
                    "summary",
                    bamana_bin,
                    &["summary", "--bam", fixture_arg(&fixture), "--full-scan"],
                    args.iterations,
                )?,
                measure_command(
                    "check_sort",
                    bamana_bin,
                    &["check_sort", "--bam", fixture_arg(&fixture), "--strict"],
                    args.iterations,
                )?,
                measure_command(
                    "check_map",
                    bamana_bin,
                    &["check_map", "--bam", fixture_arg(&fixture), "--full-scan"],
                    args.iterations,
                )?,
                measure_command(
                    "validate",
                    bamana_bin,
                    &["validate", "--bam", fixture_arg(&fixture)],
                    args.iterations,
                )?,
                measure_command(
                    "check_tag",
                    bamana_bin,
                    &[
                        "check_tag",
                        "--bam",
                        fixture_arg(&fixture),
                        "--tag",
                        "NM",
                        "--full-scan",
                        "--count-hits",
                    ],
                    args.iterations,
                )?,
                measure_command(
                    "subsample_bam",
                    bamana_bin,
                    &[
                        "subsample",
                        "--input",
                        fixture_arg(&fixture),
                        "--out",
                        fixture_arg(&subsample_output),
                        "--fraction",
                        "0.5",
                        "--mode",
                        "deterministic",
                        "--identity",
                        "full_record",
                        "--dry-run",
                        "--force",
                    ],
                    args.iterations,
                )?,
                measure_command(
                    "sort",
                    bamana_bin,
                    &[
                        "sort",
                        "--bam",
                        fixture_arg(&fixture),
                        "--out",
                        fixture_arg(&sort_output),
                        "--order",
                        "coordinate",
                        "--verify-checksum",
                        "--force",
                    ],
                    args.iterations,
                )?,
                measure_command(
                    "inspect_duplication",
                    bamana_bin,
                    &[
                        "inspect_duplication",
                        "--input",
                        fixture_arg(&fixture),
                        "--identity",
                        "qname-seq-qual-rg",
                        "--min-block-size",
                        "2",
                        "--full-scan",
                    ],
                    args.iterations,
                )?,
                measure_command(
                    "deduplicate",
                    bamana_bin,
                    &[
                        "deduplicate",
                        "--input",
                        fixture_arg(&fixture),
                        "--out",
                        fixture_arg(&deduplicate_output),
                        "--mode",
                        "contiguous-block",
                        "--identity",
                        "qname-seq-qual-rg",
                        "--min-block-size",
                        "2",
                        "--dry-run",
                        "--full-scan",
                        "--force",
                    ],
                    args.iterations,
                )?,
                measure_command(
                    "forensic_inspect",
                    bamana_bin,
                    &[
                        "forensic_inspect",
                        "--input",
                        fixture_arg(&fixture),
                        "--full-scan",
                        "--inspect-header",
                        "--inspect-rg",
                        "--inspect-pg",
                        "--inspect-readnames",
                        "--inspect-tags",
                        "--inspect-duplication",
                    ],
                    args.iterations,
                )?,
            ]
        }
        None => Vec::new(),
    };

    let mut notes = vec![
        "Fixtures are deterministic and generated locally; no private benchmark data is required."
            .to_string(),
        "Record scan throughput measures native BGZF/header opening plus full alignment-record iteration through BamScanner."
            .to_string(),
        "Selective field extraction measures scanner traversal plus core field, read-name, sequence-length, and selected aux-tag access."
            .to_string(),
        "Command timings include summary, check_sort, check_map, validate, check_tag, BAM subsample dry-run, sort with checksum verification, inspect_duplication, deduplicate dry-run, and forensic_inspect when --bamana-bin is supplied; they include process startup and JSON emission."
            .to_string(),
        "Scanner command timings are smoke timings over deterministic synthetic BAM input; they are not comparator parity claims against other tools."
            .to_string(),
        "check_sort uses strict sequential inspection, check_map and summary use full scans without adjacent index sidecars, check_tag uses full aux traversal for NM, validate uses the default full structural pass, sort uses the in-memory coordinate rewrite path with canonical checksum verification, inspect_duplication uses a full qname-seq-qual-rg CLI scan, deduplicate uses a full dry-run qname-seq-qual-rg CLI plan, and forensic_inspect uses explicit full-scan provenance scopes."
            .to_string(),
    ];
    if args.bamana_bin.is_none() {
        notes.push(
            "Command timings were skipped because --bamana-bin was not supplied.".to_string(),
        );
    }

    let report = BenchmarkReport {
        benchmark: "scanner_microbench",
        version: 1,
        profile: args.profile,
        iterations: args.iterations,
        fixture: FixtureReport {
            path: fixture.display().to_string(),
            records: fixture_model.records.len(),
            references: fixture_model.references.len(),
            sequence_len: args.profile.sequence_len(),
            serialized_payload_bytes: bam_payload.len(),
            bgzf_file_bytes,
            kept: args.keep_fixtures,
        },
        results: BenchmarkResults {
            record_scan_throughput,
            selective_field_extraction_throughput,
            command_timings,
        },
        notes,
    };

    emit_report(&report, args.out.as_deref())?;

    if !args.keep_fixtures {
        let _ = fs::remove_file(&fixture);
    }

    Ok(())
}

#[derive(Debug)]
struct FixtureModel {
    header_text: String,
    references: Vec<ReferenceRecord>,
    records: Vec<Vec<u8>>,
    record_bytes: usize,
}

fn build_fixture_model(profile: Profile) -> Result<FixtureModel, Box<dyn std::error::Error>> {
    let reference_count = profile.reference_count();
    let record_count = profile.record_count();
    let sequence_len = profile.sequence_len();

    let mut header_text = String::from("@HD\tVN:1.6\tSO:coordinate\n");
    header_text.push_str("@RG\tID:rg0\tSM:sample0\tPL:ILLUMINA\n");
    header_text.push_str("@PG\tID:bamana\tPN:bamana\tVN:0.1.0\n");
    header_text.push_str("@CO\tscanner microbenchmark fixture\n");

    let mut references = Vec::with_capacity(reference_count);
    for index in 0..reference_count {
        let name = format!("chr{index:03}");
        let length = 10_000_000 + index as u32;
        header_text.push_str(&format!("@SQ\tSN:{name}\tLN:{length}\n"));
        references.push(ReferenceRecord {
            name,
            length,
            index,
            header_fields: ReferenceHeaderFields::default(),
            text_header_length: None,
        });
    }

    let mut records = Vec::with_capacity(record_count);
    let records_per_reference = record_count.div_ceil(reference_count);
    for reference_index in 0..reference_count {
        for offset in 0..records_per_reference {
            if records.len() == record_count {
                break;
            }
            let global_index = records.len();
            let read_name = format!("read{global_index:09}");
            let pos = (offset * (sequence_len + 3)) as i32;
            let flags = if global_index % 97 == 0 { 0x400 } else { 0 };
            let mapq = 20 + (global_index % 41) as u8;
            records.push(build_record(
                reference_index as i32,
                pos,
                &read_name,
                flags,
                mapq,
                sequence_len,
                global_index,
            )?);
        }
    }

    let record_bytes = records.iter().map(Vec::len).sum();
    Ok(FixtureModel {
        header_text,
        references,
        records,
        record_bytes,
    })
}

fn build_record(
    ref_id: i32,
    pos: i32,
    read_name: &str,
    flags: u16,
    mapq: u8,
    sequence_len: usize,
    record_index: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let sequence = synthetic_sequence(sequence_len, record_index);
    let sequence_bytes = encode_bam_sequence(&sequence)?;
    let qualities = "I".repeat(sequence_len);
    let quality_bytes = encode_bam_qualities(&qualities)?;
    let cigar_bytes = vec![((sequence_len as u32) << 4).to_le_bytes()];
    let cigar_bytes = cigar_bytes.into_iter().flatten().collect::<Vec<_>>();

    let mut aux_bytes = Vec::new();
    aux_bytes.extend_from_slice(b"RGZrg0\0");
    aux_bytes.extend_from_slice(b"NMc");
    aux_bytes.push((record_index % 8) as u8);

    let mut variable = Vec::new();
    variable.extend_from_slice(read_name.as_bytes());
    variable.push(0);
    variable.extend_from_slice(&cigar_bytes);
    variable.extend_from_slice(&sequence_bytes);
    variable.extend_from_slice(&quality_bytes);
    variable.extend_from_slice(&aux_bytes);

    let block_size = 32 + variable.len();
    let bin_mq_nl = ((4680_u32) << 16) | ((mapq as u32) << 8) | ((read_name.len() + 1) as u32);
    let flag_nc = ((flags as u32) << 16) | 1;

    let mut record = Vec::with_capacity(4 + block_size);
    record.extend_from_slice(&(block_size as i32).to_le_bytes());
    record.extend_from_slice(&ref_id.to_le_bytes());
    record.extend_from_slice(&pos.to_le_bytes());
    record.extend_from_slice(&bin_mq_nl.to_le_bytes());
    record.extend_from_slice(&flag_nc.to_le_bytes());
    record.extend_from_slice(&(sequence_len as i32).to_le_bytes());
    record.extend_from_slice(&(-1_i32).to_le_bytes());
    record.extend_from_slice(&(-1_i32).to_le_bytes());
    record.extend_from_slice(&0_i32.to_le_bytes());
    record.extend_from_slice(&variable);
    Ok(record)
}

fn synthetic_sequence(len: usize, seed: usize) -> String {
    const BASES: &[u8] = b"ACGT";
    let mut sequence = String::with_capacity(len);
    for index in 0..len {
        sequence.push(BASES[(seed + index) % BASES.len()] as char);
    }
    sequence
}

fn default_workdir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bamana-scanner-microbench-{}-{nonce}",
        std::process::id()
    ))
}

fn write_bgzf_payload(path: &Path, payload: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BgzfWriter::create(path)?;
    writer.write_all(payload)?;
    writer.finish()?;
    Ok(())
}

fn measure_record_scan(
    fixture: &Path,
    file_bytes: u64,
    iterations: usize,
) -> Result<Measurement, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let mut records_per_iteration = 0_u64;
    let mut checksum = 0_u64;

    for _ in 0..iterations {
        let mut scanner = BamScanner::open(fixture)?;
        let mut records = 0_u64;
        let mut iteration_checksum = 0_u64;
        let started = Instant::now();
        while let Some(record) = scanner.next_record()? {
            records += 1;
            iteration_checksum = iteration_checksum.wrapping_add(record.block_size() as u64);
        }
        samples.push(started.elapsed().as_secs_f64());
        records_per_iteration = records;
        checksum ^= iteration_checksum;
    }

    Ok(measurement(
        "record_scan_throughput",
        iterations,
        records_per_iteration,
        file_bytes,
        checksum,
        &samples,
    ))
}

fn measure_selective_extraction(
    fixture: &Path,
    file_bytes: u64,
    iterations: usize,
) -> Result<Measurement, Box<dyn std::error::Error>> {
    let query = TagQuery {
        tag: *b"NM",
        required_type: Some(AuxTypeCode::CLower),
    };
    let mut samples = Vec::with_capacity(iterations);
    let mut records_per_iteration = 0_u64;
    let mut checksum = 0_u64;

    for _ in 0..iterations {
        let mut scanner = BamScanner::open(fixture)?;
        let mut records = 0_u64;
        let mut iteration_checksum = 0_u64;
        let started = Instant::now();
        while let Some(record) = scanner.next_record()? {
            records += 1;
            let flags = record.flag_summary();
            let matched = record_aux_contains_tag(&record, query)?;
            iteration_checksum = iteration_checksum
                .wrapping_add(record.ref_id() as u64)
                .wrapping_add(record.pos() as u64)
                .wrapping_add(flags.raw as u64)
                .wrapping_add(record.mapping_quality() as u64)
                .wrapping_add(record.sequence_len() as u64)
                .wrapping_add(record.read_name().len() as u64)
                .wrapping_add(u64::from(matched));
        }
        samples.push(started.elapsed().as_secs_f64());
        records_per_iteration = records;
        checksum ^= iteration_checksum;
    }

    Ok(measurement(
        "selective_field_extraction_throughput",
        iterations,
        records_per_iteration,
        file_bytes,
        checksum,
        &samples,
    ))
}

fn measure_command(
    command_name: &'static str,
    bamana_bin: &Path,
    args: &[&str],
    iterations: usize,
) -> Result<CommandTiming, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let mut exit_codes = Vec::with_capacity(iterations);
    let mut ok_count = 0;

    for _ in 0..iterations {
        let started = Instant::now();
        let output = Command::new(bamana_bin)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        samples.push(started.elapsed().as_secs_f64());
        exit_codes.push(output.status.code());
        if output.status.success() {
            ok_count += 1;
        }
    }

    let (total_seconds, min_seconds, mean_seconds, max_seconds) = summarize(&samples);
    Ok(CommandTiming {
        command: command_name,
        iterations,
        ok_count,
        total_seconds,
        min_seconds,
        mean_seconds,
        max_seconds,
        exit_codes,
    })
}

fn fixture_arg(fixture: &Path) -> &str {
    fixture
        .to_str()
        .expect("benchmark fixture path should be valid UTF-8")
}

fn measurement(
    operation: &'static str,
    iterations: usize,
    records_per_iteration: u64,
    bytes_per_iteration: u64,
    checksum: u64,
    samples: &[f64],
) -> Measurement {
    let (total_seconds, min_seconds, mean_seconds, max_seconds) = summarize(samples);
    let records_per_second = if total_seconds > 0.0 {
        (records_per_iteration as f64 * iterations as f64) / total_seconds
    } else {
        0.0
    };
    let bytes_per_second = if total_seconds > 0.0 {
        (bytes_per_iteration as f64 * iterations as f64) / total_seconds
    } else {
        0.0
    };

    Measurement {
        operation,
        iterations,
        records_per_iteration,
        bytes_per_iteration,
        total_seconds,
        min_seconds,
        mean_seconds,
        max_seconds,
        records_per_second,
        bytes_per_second,
        checksum,
    }
}

fn summarize(samples: &[f64]) -> (f64, f64, f64, f64) {
    let total_seconds: f64 = samples.iter().sum();
    let min_seconds = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let max_seconds = samples.iter().copied().fold(0.0, f64::max);
    let mean_seconds = total_seconds / samples.len() as f64;
    (total_seconds, min_seconds, mean_seconds, max_seconds)
}

fn emit_report(
    report: &BenchmarkReport,
    out: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(report)?;
    match out {
        Some(path) => fs::write(path, format!("{json}\n"))?,
        None => {
            let mut stdout = io::stdout().lock();
            writeln!(stdout, "{json}")?;
        }
    }
    Ok(())
}
