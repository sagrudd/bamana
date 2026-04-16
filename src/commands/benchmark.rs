use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;

use crate::{cli::BenchmarkProfile, error::AppError, json::CommandResponse};

#[derive(Debug)]
pub struct BenchmarkRequest {
    pub profile: BenchmarkProfile,
    pub fastq: PathBuf,
    pub bam: PathBuf,
    pub report: PathBuf,
    pub threads: usize,
    pub container_image: String,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkPayload {
    pub profile: String,
    pub fastq: String,
    pub bamana_bam: String,
    pub comparator_bam: String,
    pub report_pdf: String,
    pub workdir: String,
    pub raw_results_dir: String,
    pub aggregated_dir: String,
    pub metadata_dir: String,
    pub logs_dir: String,
    pub container_image: String,
    pub bamana_binary: String,
    pub tool_versions_tsv: String,
    pub steps: Vec<BenchmarkStep>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkStep {
    pub name: String,
    pub log_path: String,
    pub command: String,
}

pub fn run(request: BenchmarkRequest) -> CommandResponse<BenchmarkPayload> {
    let fastq = request.fastq.clone();
    match run_impl(&request) {
        Ok(payload) => CommandResponse::success("benchmark", Some(fastq.as_path()), payload),
        Err(error) => CommandResponse::failure("benchmark", Some(fastq.as_path()), error),
    }
}

fn run_impl(request: &BenchmarkRequest) -> Result<BenchmarkPayload, AppError> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let current_dir = env::current_dir().map_err(|error| AppError::Io {
        path: PathBuf::from("."),
        message: error.to_string(),
    })?;

    let fastq = absolute_existing_path(&current_dir, &request.fastq)?;
    let bam = absolute_path(&current_dir, &request.bam);
    let report = absolute_path(&current_dir, &request.report);

    ensure_parent_dir(&bam)?;
    ensure_parent_dir(&report)?;

    let comparator_bam = comparator_bam_path(&bam);
    let workdir = report
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{}.benchmark", file_stem_or_name(&report)));

    let raw_results_dir = workdir.join("raw");
    let aggregated_dir = workdir.join("aggregated");
    let metadata_dir = workdir.join("metadata");
    let logs_dir = workdir.join("logs");
    let tool_versions_tsv = metadata_dir.join("tool_versions.tsv");
    let bamana_binary = repo_root.join("target").join("release").join("bamana");

    if !request.force {
        for path in [&bam, &comparator_bam, &report, &workdir] {
            if path.exists() {
                return Err(AppError::OutputExists {
                    path: path.to_path_buf(),
                });
            }
        }
    }

    if workdir.exists() {
        fs::remove_dir_all(&workdir).map_err(|error| AppError::Io {
            path: workdir.clone(),
            message: error.to_string(),
        })?;
    }
    for path in [&bam, &comparator_bam, &report] {
        if path.exists() {
            fs::remove_file(path).map_err(|error| AppError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        }
    }

    fs::create_dir_all(&raw_results_dir).map_err(|error| AppError::Io {
        path: raw_results_dir.clone(),
        message: error.to_string(),
    })?;
    fs::create_dir_all(&aggregated_dir).map_err(|error| AppError::Io {
        path: aggregated_dir.clone(),
        message: error.to_string(),
    })?;
    fs::create_dir_all(&metadata_dir).map_err(|error| AppError::Io {
        path: metadata_dir.clone(),
        message: error.to_string(),
    })?;
    fs::create_dir_all(&logs_dir).map_err(|error| AppError::Io {
        path: logs_dir.clone(),
        message: error.to_string(),
    })?;

    let cargo_log = logs_dir.join("cargo_build.log");
    let cargo_args = vec![
        "build".to_string(),
        "--release".to_string(),
        "--bin".to_string(),
        "bamana".to_string(),
    ];
    run_and_log("cargo", &cargo_args, &repo_root, &cargo_log, "cargo build")?;

    let docker_build_log = logs_dir.join("docker_build.log");
    let docker_build_args = vec![
        "build".to_string(),
        "-f".to_string(),
        "benchmarks/Dockerfile".to_string(),
        "-t".to_string(),
        request.container_image.clone(),
        ".".to_string(),
    ];
    run_and_log(
        "docker",
        &docker_build_args,
        &repo_root,
        &docker_build_log,
        "docker build",
    )?;

    let fastq_parent = fastq.parent().unwrap_or_else(|| Path::new("/"));
    let bam_parent = bam.parent().unwrap_or_else(|| Path::new("/"));
    let report_parent = report.parent().unwrap_or_else(|| Path::new("/"));

    let fastq_container = container_join("/benchmark-inputs", file_name(&fastq)?);
    let bam_container = container_join("/benchmark-outputs", file_name(&bam)?);
    let comparator_container = container_join("/benchmark-outputs", file_name(&comparator_bam)?);
    let report_container = container_join("/benchmark-report", file_name(&report)?);
    let workdir_container = container_join(
        "/benchmark-report",
        workdir
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| AppError::Internal {
                message: format!("invalid workdir name: {}", workdir.display()),
            })?,
    );

    let container_run_log = logs_dir.join("container_run.log");
    let docker_user = current_user_spec(&repo_root)?;
    let docker_run_args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "--user".to_string(),
        docker_user,
        "-v".to_string(),
        format!("{}:/workspace", repo_root.display()),
        "-v".to_string(),
        format!("{}:/benchmark-inputs:ro", fastq_parent.display()),
        "-v".to_string(),
        format!("{}:/benchmark-outputs", bam_parent.display()),
        "-v".to_string(),
        format!("{}:/benchmark-report", report_parent.display()),
        request.container_image.clone(),
        "bash".to_string(),
        "/workspace/benchmarks/bin/run_fastq_ingress_benchmark.sh".to_string(),
        "--profile".to_string(),
        profile_id(request.profile).to_string(),
        "--fastq".to_string(),
        fastq_container.clone(),
        "--bamana-output".to_string(),
        bam_container.clone(),
        "--fastcat-samtools-output".to_string(),
        comparator_container.clone(),
        "--report".to_string(),
        report_container.clone(),
        "--workdir".to_string(),
        workdir_container.clone(),
        "--threads".to_string(),
        request.threads.max(1).to_string(),
        "--container-image".to_string(),
        request.container_image.clone(),
        "--bamana-bin".to_string(),
        "/workspace/target/release/bamana".to_string(),
    ];

    run_and_log(
        "docker",
        &docker_run_args,
        &repo_root,
        &container_run_log,
        "docker run",
    )?;

    Ok(BenchmarkPayload {
        profile: profile_id(request.profile).to_string(),
        fastq: fastq.display().to_string(),
        bamana_bam: bam.display().to_string(),
        comparator_bam: comparator_bam.display().to_string(),
        report_pdf: report.display().to_string(),
        workdir: workdir.display().to_string(),
        raw_results_dir: raw_results_dir.display().to_string(),
        aggregated_dir: aggregated_dir.display().to_string(),
        metadata_dir: metadata_dir.display().to_string(),
        logs_dir: logs_dir.display().to_string(),
        container_image: request.container_image.clone(),
        bamana_binary: bamana_binary.display().to_string(),
        tool_versions_tsv: tool_versions_tsv.display().to_string(),
        steps: vec![
            BenchmarkStep {
                name: "build_bamana".to_string(),
                log_path: cargo_log.display().to_string(),
                command: format!("cargo {}", cargo_args.join(" ")),
            },
            BenchmarkStep {
                name: "build_container".to_string(),
                log_path: docker_build_log.display().to_string(),
                command: format!("docker {}", docker_build_args.join(" ")),
            },
            BenchmarkStep {
                name: "run_profile".to_string(),
                log_path: container_run_log.display().to_string(),
                command: format!("docker {}", docker_run_args.join(" ")),
            },
        ],
        notes: vec![
            "The fastq_ingress profile compares Bamana against a fastcat-plus-samtools unmapped-BAM path.".to_string(),
            "The PDF report is rendered from R Markdown inside the benchmark container.".to_string(),
        ],
    })
}

fn current_user_spec(cwd: &Path) -> Result<String, AppError> {
    let uid = capture_single_line("id", &["-u"], cwd, "resolve current uid")?;
    let gid = capture_single_line("id", &["-g"], cwd, "resolve current gid")?;
    Ok(format!("{uid}:{gid}"))
}

fn absolute_existing_path(current_dir: &Path, path: &Path) -> Result<PathBuf, AppError> {
    let absolute = absolute_path(current_dir, path);
    if !absolute.exists() {
        return Err(AppError::FileNotFound { path: absolute });
    }
    Ok(absolute)
}

fn absolute_path(current_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    Ok(())
}

fn comparator_bam_path(bam: &Path) -> PathBuf {
    let stem = file_stem_or_name(bam);
    let file_name = format!("{stem}.fastcat_samtools.bam");
    bam.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name)
}

fn file_name(path: &Path) -> Result<&str, AppError> {
    path.file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| AppError::Internal {
            message: format!("invalid file name: {}", path.display()),
        })
}

fn file_stem_or_name(path: &Path) -> String {
    path.file_stem()
        .and_then(OsStr::to_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "benchmark".to_string())
}

fn container_join(prefix: &str, name: &str) -> String {
    format!("{}/{}", prefix.trim_end_matches('/'), name)
}

fn profile_id(profile: BenchmarkProfile) -> &'static str {
    match profile {
        BenchmarkProfile::FastqIngress => "fastq_ingress",
    }
}

fn run_and_log(
    program: &str,
    args: &[String],
    cwd: &Path,
    log_path: &Path,
    label: &str,
) -> Result<(), AppError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| AppError::Io {
            path: cwd.to_path_buf(),
            message: format!("failed to launch {label}: {error}"),
        })?;

    let mut body = String::new();
    body.push_str("$ ");
    body.push_str(program);
    if !args.is_empty() {
        body.push(' ');
        body.push_str(&args.join(" "));
    }
    body.push('\n');
    body.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        if !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    fs::write(log_path, body).map_err(|error| AppError::Io {
        path: log_path.to_path_buf(),
        message: error.to_string(),
    })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(AppError::Internal {
            message: format!(
                "{label} failed with exit code {:?}. See {}.",
                output.status.code(),
                log_path.display()
            ),
        })
    }
}

fn capture_single_line(
    program: &str,
    args: &[&str],
    cwd: &Path,
    label: &str,
) -> Result<String, AppError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| AppError::Io {
            path: cwd.to_path_buf(),
            message: format!("failed to launch {label}: {error}"),
        })?;

    if !output.status.success() {
        return Err(AppError::Internal {
            message: format!("{label} failed with exit code {:?}.", output.status.code()),
        });
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return Err(AppError::Internal {
            message: format!("{label} returned an empty value."),
        });
    }
    Ok(value)
}
