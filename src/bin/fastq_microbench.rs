use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use bamana::fastq::{FastqRecord, count_fastq_records, write_fastq_records};
use clap::{Parser, ValueEnum};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "fastq_microbench",
    about = "Run Bamana-native FASTQ parser and writer microbenchmarks and emit JSON results."
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
    plain_path: String,
    gzip_path: String,
    records: usize,
    sequence_len: usize,
    plain_file_bytes: u64,
    gzip_file_bytes: u64,
    kept: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkResults {
    plain_parse_throughput: Measurement,
    gzip_parse_throughput: Measurement,
    plain_writer_throughput: Measurement,
    gzip_writer_throughput: Measurement,
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

    let records = build_records(args.profile)?;
    let plain_fixture =
        workdir.join(format!("fastq-microbench-{:?}.fastq", args.profile).to_lowercase());
    let gzip_fixture =
        workdir.join(format!("fastq-microbench-{:?}.fastq.gz", args.profile).to_lowercase());
    write_fastq_records(&plain_fixture, &records)?;
    write_fastq_records(&gzip_fixture, &records)?;

    let plain_file_bytes = fs::metadata(&plain_fixture)?.len();
    let gzip_file_bytes = fs::metadata(&gzip_fixture)?.len();

    let plain_parse_throughput = measure_parse(
        "plain_parse_throughput",
        &plain_fixture,
        plain_file_bytes,
        args.iterations,
    )?;
    let gzip_parse_throughput = measure_parse(
        "gzip_parse_throughput",
        &gzip_fixture,
        gzip_file_bytes,
        args.iterations,
    )?;
    let plain_writer_throughput = measure_writer(
        "plain_writer_throughput",
        &records,
        &workdir,
        "fastq",
        plain_file_bytes,
        args.iterations,
    )?;
    let gzip_writer_throughput = measure_writer(
        "gzip_writer_throughput",
        &records,
        &workdir,
        "fastq.gz",
        gzip_file_bytes,
        args.iterations,
    )?;
    let command_timings = match args.bamana_bin.as_deref() {
        Some(bamana_bin) => {
            let plain_subsample_output = workdir.join("fastq-microbench-subsample-out.fastq");
            let gzip_subsample_output = workdir.join("fastq-microbench-subsample-out.fastq.gz");
            vec![
                measure_command(
                    "enumerate_fastq",
                    bamana_bin,
                    &["enumerate", "--input", fixture_arg(&plain_fixture)],
                    args.iterations,
                )?,
                measure_command(
                    "enumerate_fastq_gz",
                    bamana_bin,
                    &["enumerate", "--input", fixture_arg(&gzip_fixture)],
                    args.iterations,
                )?,
                measure_command(
                    "subsample_fastq",
                    bamana_bin,
                    &[
                        "subsample",
                        "--input",
                        fixture_arg(&plain_fixture),
                        "--out",
                        fixture_arg(&plain_subsample_output),
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
                    "subsample_fastq_gz",
                    bamana_bin,
                    &[
                        "subsample",
                        "--input",
                        fixture_arg(&gzip_fixture),
                        "--out",
                        fixture_arg(&gzip_subsample_output),
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
            ]
        }
        None => Vec::new(),
    };

    let mut notes = vec![
        "Fixtures are deterministic and generated locally; no private benchmark data is required."
            .to_string(),
        "Parse throughput measures the Bamana-native FASTQ reader facade and record validation."
            .to_string(),
        "Writer throughput measures the Bamana-native FASTQ writer facade, including gzip finalization for gzip output."
            .to_string(),
        "Command timings include enumerate and FASTQ/FASTQ.GZ subsample dry-runs when --bamana-bin is supplied; they include process startup and JSON emission."
            .to_string(),
    ];
    if args.bamana_bin.is_none() {
        notes.push(
            "Command timings were skipped because --bamana-bin was not supplied.".to_string(),
        );
    }

    let report = BenchmarkReport {
        benchmark: "fastq_microbench",
        version: 1,
        profile: args.profile,
        iterations: args.iterations,
        fixture: FixtureReport {
            plain_path: plain_fixture.display().to_string(),
            gzip_path: gzip_fixture.display().to_string(),
            records: records.len(),
            sequence_len: args.profile.sequence_len(),
            plain_file_bytes,
            gzip_file_bytes,
            kept: args.keep_fixtures,
        },
        results: BenchmarkResults {
            plain_parse_throughput,
            gzip_parse_throughput,
            plain_writer_throughput,
            gzip_writer_throughput,
            command_timings,
        },
        notes,
    };

    emit_report(&report, args.out.as_deref())?;

    if !args.keep_fixtures {
        let _ = fs::remove_file(&plain_fixture);
        let _ = fs::remove_file(&gzip_fixture);
    }

    Ok(())
}

fn build_records(profile: Profile) -> Result<Vec<FastqRecord>, Box<dyn std::error::Error>> {
    let record_count = profile.record_count();
    let sequence_len = profile.sequence_len();
    let mut records = Vec::with_capacity(record_count);
    let alphabet = b"ACGT";
    let quality = "I".repeat(sequence_len);

    for index in 0..record_count {
        let mut sequence = String::with_capacity(sequence_len);
        for offset in 0..sequence_len {
            let base = alphabet[(index + offset) % alphabet.len()] as char;
            sequence.push(base);
        }
        records.push(
            FastqRecord::from_lines(
                format!("@read{index:08} run=fastq_microbench"),
                sequence,
                "+synthetic".to_string(),
                quality.clone(),
            )
            .map_err(|error| {
                format!(
                    "generated FASTQ record did not validate: {}",
                    error.detail()
                )
            })?,
        );
    }

    Ok(records)
}

fn default_workdir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bamana-fastq-microbench-{}-{nonce}",
        std::process::id()
    ))
}

fn measure_parse(
    operation: &'static str,
    fixture: &Path,
    file_bytes: u64,
    iterations: usize,
) -> Result<Measurement, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let mut checksum = 0_u64;

    for _ in 0..iterations {
        let started = Instant::now();
        let count = count_fastq_records(fixture)?;
        samples.push(started.elapsed().as_secs_f64());
        checksum ^= count;
    }

    Ok(measurement(
        operation,
        iterations,
        count_fastq_records(fixture)?,
        file_bytes,
        checksum,
        &samples,
    ))
}

fn measure_writer(
    operation: &'static str,
    records: &[FastqRecord],
    workdir: &Path,
    extension: &str,
    expected_bytes: u64,
    iterations: usize,
) -> Result<Measurement, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let mut checksum = 0_u64;

    for iteration in 0..iterations {
        let path = workdir.join(format!("fastq-writer-{operation}-{iteration}.{extension}"));
        let started = Instant::now();
        write_fastq_records(&path, records)?;
        samples.push(started.elapsed().as_secs_f64());
        checksum ^= fs::metadata(&path)?.len();
        let _ = fs::remove_file(path);
    }

    Ok(measurement(
        operation,
        iterations,
        records.len() as u64,
        expected_bytes,
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
            stdout.write_all(json.as_bytes())?;
            stdout.write_all(b"\n")?;
        }
    }
    Ok(())
}
