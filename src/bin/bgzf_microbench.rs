use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use bamana::bgzf::{BgzfWriter, has_bgzf_eof, read_bgzf_payloads};
use clap::{Parser, ValueEnum};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "bgzf_microbench",
    about = "Run Bamana-native BGZF microbenchmarks and emit JSON results."
)]
struct Args {
    #[arg(long = "profile", value_enum, default_value_t = Profile::Small)]
    profile: Profile,
    #[arg(long = "iterations", default_value_t = 5)]
    iterations: usize,
    #[arg(short = 'j', long = "threads", default_value_t = 1)]
    threads: usize,
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
    fn payload_bytes(self) -> usize {
        match self {
            Self::Small => 128 * 1024,
            Self::Medium => 8 * 1024 * 1024,
            Self::Large => 128 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    benchmark: &'static str,
    version: u32,
    profile: Profile,
    iterations: usize,
    threads: usize,
    fixture: FixtureReport,
    results: BenchmarkResults,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FixtureReport {
    path: String,
    payload_bytes: usize,
    bgzf_file_bytes: u64,
    kept: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkResults {
    write_throughput: Measurement,
    read_throughput: Measurement,
    eof_check_latency: Measurement,
    command_timings: Vec<CommandTiming>,
}

#[derive(Debug, Serialize)]
struct Measurement {
    operation: &'static str,
    iterations: usize,
    bytes_per_iteration: u64,
    total_seconds: f64,
    min_seconds: f64,
    mean_seconds: f64,
    max_seconds: f64,
    bytes_per_second: f64,
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
    if args.threads == 0 {
        return Err("--threads must be greater than zero".into());
    }

    let workdir = args.workdir.unwrap_or_else(default_workdir);
    fs::create_dir_all(&workdir)?;

    let payload = build_bam_like_payload(args.profile.payload_bytes());
    let fixture = workdir.join(format!("bgzf-microbench-{:?}.bam", args.profile).to_lowercase());
    write_bgzf_payload(&fixture, &payload, args.threads)?;
    let bgzf_file_bytes = fs::metadata(&fixture)?.len();

    let write_throughput = measure_write(&workdir, &payload, args.iterations, args.threads)?;
    let read_throughput = measure_read(&fixture, args.iterations)?;
    let eof_check_latency = measure_eof(&fixture, args.iterations)?;
    let command_timings = match args.bamana_bin.as_deref() {
        Some(bamana_bin) => vec![
            measure_command("verify", bamana_bin, &fixture, args.iterations)?,
            measure_command("check_eof", bamana_bin, &fixture, args.iterations)?,
        ],
        None => Vec::new(),
    };

    let mut notes = vec![
        "Fixtures are deterministic and generated locally; no private benchmark data is required."
            .to_string(),
        "Read throughput measures native BGZF member inflation into memory.".to_string(),
        "Write throughput measures native BGZF member emission to a temporary file.".to_string(),
        "Command timings include verify and check_eof when --bamana-bin is supplied; they include process startup and JSON emission."
            .to_string(),
        "check_eof command timing is EOF-marker smoke evidence only; it does not measure BAM header parsing or alignment-record validation."
            .to_string(),
    ];
    if args.bamana_bin.is_none() {
        notes.push(
            "Command timings were skipped because --bamana-bin was not supplied.".to_string(),
        );
    }

    let report = BenchmarkReport {
        benchmark: "bgzf_microbench",
        version: 1,
        profile: args.profile,
        iterations: args.iterations,
        threads: args.threads,
        fixture: FixtureReport {
            path: fixture.display().to_string(),
            payload_bytes: payload.len(),
            bgzf_file_bytes,
            kept: args.keep_fixtures,
        },
        results: BenchmarkResults {
            write_throughput,
            read_throughput,
            eof_check_latency,
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

fn default_workdir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bamana-bgzf-microbench-{}-{nonce}",
        std::process::id()
    ))
}

fn build_bam_like_payload(target_bytes: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(target_bytes.max(12));
    payload.extend_from_slice(b"BAM\x01");
    payload.extend_from_slice(&0_i32.to_le_bytes());
    payload.extend_from_slice(&0_i32.to_le_bytes());

    let mut state = 0x9e37_79b9_7f4a_7c15_u64 ^ target_bytes as u64;
    while payload.len() < target_bytes {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        payload.push((state >> 32) as u8);
    }

    payload
}

fn write_bgzf_payload(
    path: &Path,
    payload: &[u8],
    threads: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BgzfWriter::create_with_threads(path, threads)?;
    writer.write_all(payload)?;
    writer.finish()?;
    Ok(())
}

fn measure_write(
    workdir: &Path,
    payload: &[u8],
    iterations: usize,
    threads: usize,
) -> Result<Measurement, Box<dyn std::error::Error>> {
    let path = workdir.join("bgzf-microbench-write.bam");
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let started = Instant::now();
        write_bgzf_payload(&path, payload, threads)?;
        samples.push(started.elapsed().as_secs_f64());
    }

    let _ = fs::remove_file(path);
    Ok(measurement(
        "write_throughput",
        iterations,
        payload.len() as u64,
        &samples,
    ))
}

fn measure_read(path: &Path, iterations: usize) -> Result<Measurement, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let mut observed_bytes = 0_u64;

    for _ in 0..iterations {
        let started = Instant::now();
        let payloads = read_bgzf_payloads(path)?;
        let elapsed = started.elapsed().as_secs_f64();
        observed_bytes = payloads
            .iter()
            .map(|payload| payload.len() as u64)
            .sum::<u64>();
        samples.push(elapsed);
    }

    Ok(measurement(
        "read_throughput",
        iterations,
        observed_bytes,
        &samples,
    ))
}

fn measure_eof(path: &Path, iterations: usize) -> Result<Measurement, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let file_bytes = fs::metadata(path)?.len();

    for _ in 0..iterations {
        let started = Instant::now();
        if !has_bgzf_eof(path)? {
            return Err(format!(
                "{} did not contain a canonical BGZF EOF marker",
                path.display()
            )
            .into());
        }
        samples.push(started.elapsed().as_secs_f64());
    }

    Ok(measurement(
        "eof_check_latency",
        iterations,
        file_bytes,
        &samples,
    ))
}

fn measure_command(
    command_name: &'static str,
    bamana_bin: &Path,
    fixture: &Path,
    iterations: usize,
) -> Result<CommandTiming, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let mut exit_codes = Vec::with_capacity(iterations);
    let mut ok_count = 0;

    for _ in 0..iterations {
        let started = Instant::now();
        let output = Command::new(bamana_bin)
            .arg(command_name)
            .arg("--bam")
            .arg(fixture)
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

fn measurement(
    operation: &'static str,
    iterations: usize,
    bytes_per_iteration: u64,
    samples: &[f64],
) -> Measurement {
    let (total_seconds, min_seconds, mean_seconds, max_seconds) = summarize(samples);
    let bytes_per_second = if total_seconds > 0.0 {
        (bytes_per_iteration as f64 * iterations as f64) / total_seconds
    } else {
        0.0
    };

    Measurement {
        operation,
        iterations,
        bytes_per_iteration,
        total_seconds,
        min_seconds,
        mean_seconds,
        max_seconds,
        bytes_per_second,
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
