use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use bamana::{
    bam::header::{
        ReferenceHeaderFields, ReferenceRecord, parse_bam_header_from_native_bgzf,
        serialize_bam_header_payload, serialize_bam_header_view_payload,
    },
    bgzf::BgzfWriter,
};
use clap::{Parser, ValueEnum};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "header_microbench",
    about = "Run Bamana-native BAM header microbenchmarks and emit JSON results."
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
    fn reference_count(self) -> usize {
        match self {
            Self::Small => 2,
            Self::Medium => 128,
            Self::Large => 4096,
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
    header_text_bytes: usize,
    reference_count: usize,
    serialized_header_bytes: usize,
    bgzf_file_bytes: u64,
    kept: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkResults {
    header_parse_latency: Measurement,
    header_serialization_latency: Measurement,
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

    let workdir = args.workdir.unwrap_or_else(default_workdir);
    fs::create_dir_all(&workdir)?;

    let fixture_model = build_fixture_model(args.profile);
    let header_payload = serialize_bam_header_payload(
        &workdir.join("header-microbench.bam"),
        &fixture_model.header_text,
        &fixture_model.references,
    )?;
    let fixture = workdir.join(format!("header-microbench-{:?}.bam", args.profile).to_lowercase());
    write_bgzf_payload(&fixture, &header_payload)?;
    let bgzf_file_bytes = fs::metadata(&fixture)?.len();

    let parsed_header = parse_bam_header_from_native_bgzf(&fixture)?;
    let header_parse_latency = measure_header_parse(&fixture, args.iterations)?;
    let header_serialization_latency = measure_header_serialization(
        &fixture,
        &parsed_header.header,
        header_payload.len(),
        args.iterations,
    )?;
    let command_timings = match args.bamana_bin.as_deref() {
        Some(bamana_bin) => vec![
            measure_command("verify", bamana_bin, &fixture, args.iterations)?,
            measure_command("header", bamana_bin, &fixture, args.iterations)?,
            measure_command("reheader", bamana_bin, &fixture, args.iterations)?,
        ],
        None => Vec::new(),
    };

    let mut notes = vec![
        "Fixtures are deterministic and generated locally; no private benchmark data is required."
            .to_string(),
        "Header parse latency measures native BGZF streaming plus native BAM header parsing."
            .to_string(),
        "Header serialization latency measures deterministic BAM header payload serialization without writing BGZF blocks."
            .to_string(),
    ];
    if args.bamana_bin.is_none() {
        notes.push(
            "Command timings were skipped because --bamana-bin was not supplied.".to_string(),
        );
    }

    let report = BenchmarkReport {
        benchmark: "header_microbench",
        version: 1,
        profile: args.profile,
        iterations: args.iterations,
        fixture: FixtureReport {
            path: fixture.display().to_string(),
            header_text_bytes: fixture_model.header_text.len(),
            reference_count: fixture_model.references.len(),
            serialized_header_bytes: header_payload.len(),
            bgzf_file_bytes,
            kept: args.keep_fixtures,
        },
        results: BenchmarkResults {
            header_parse_latency,
            header_serialization_latency,
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
}

fn build_fixture_model(profile: Profile) -> FixtureModel {
    let reference_count = profile.reference_count();
    let mut header_text = String::from("@HD\tVN:1.6\tSO:coordinate\n");
    header_text.push_str("@RG\tID:rg0\tSM:sample0\tPL:ILLUMINA\n");
    header_text.push_str("@PG\tID:bamana\tPN:bamana\tVN:0.1.0\n");
    header_text.push_str("@CO\theader microbenchmark fixture\n");

    let mut references = Vec::with_capacity(reference_count);
    for index in 0..reference_count {
        let name = format!("chr{index:05}");
        let length = 1_000_000 + index as u32;
        header_text.push_str(&format!(
            "@SQ\tSN:{name}\tLN:{length}\tM5:{:032x}\tUR:file://ref/{name}.fa\n",
            index
        ));
        references.push(ReferenceRecord {
            name,
            length,
            index,
            header_fields: ReferenceHeaderFields::default(),
            text_header_length: None,
        });
    }

    FixtureModel {
        header_text,
        references,
    }
}

fn default_workdir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "bamana-header-microbench-{}-{nonce}",
        std::process::id()
    ))
}

fn write_bgzf_payload(path: &Path, payload: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BgzfWriter::create(path)?;
    writer.write_all(payload)?;
    writer.finish()?;
    Ok(())
}

fn measure_header_parse(
    fixture: &Path,
    iterations: usize,
) -> Result<Measurement, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);
    let file_bytes = fs::metadata(fixture)?.len();

    for _ in 0..iterations {
        let started = Instant::now();
        parse_bam_header_from_native_bgzf(fixture)?;
        samples.push(started.elapsed().as_secs_f64());
    }

    Ok(measurement(
        "header_parse_latency",
        iterations,
        file_bytes,
        &samples,
    ))
}

fn measure_header_serialization(
    fixture: &Path,
    header: &bamana::bam::header::BamHeaderView,
    header_payload_bytes: usize,
    iterations: usize,
) -> Result<Measurement, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let started = Instant::now();
        serialize_bam_header_view_payload(fixture, header)?;
        samples.push(started.elapsed().as_secs_f64());
    }

    Ok(measurement(
        "header_serialization_latency",
        iterations,
        header_payload_bytes as u64,
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
        let mut command = Command::new(bamana_bin);
        command.arg(command_name).arg("--bam").arg(fixture);
        if command_name == "reheader" {
            command
                .arg("--add-comment")
                .arg("header_microbench smoke")
                .arg("--dry-run")
                .arg("--rewrite-minimized");
        }
        let output = command
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
