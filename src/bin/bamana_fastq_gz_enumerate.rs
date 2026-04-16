use std::{env, fs, path::PathBuf, process::ExitCode};

use bamana::{error::AppError, fastq::count_fastq_records, json::JsonError};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct EnumeratePayload {
    input: String,
    records: u64,
}

#[derive(Debug, Serialize)]
struct EnumerateResponse {
    ok: bool,
    data: Option<EnumeratePayload>,
    error: Option<JsonError>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let body = serde_json::to_string(&EnumerateResponse {
                ok: false,
                data: None,
                error: Some(error.to_json_error()),
            })
            .unwrap_or_else(|_| "{\"ok\":false}".to_string());
            eprintln!("{body}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), AppError> {
    let mut input = None;
    let mut out = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = args.next().map(PathBuf::from),
            "--out" => out = args.next().map(PathBuf::from),
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => {
                return Err(AppError::Internal {
                    message: format!("unknown argument: {other}"),
                });
            }
        }
    }

    let input = input.ok_or_else(|| AppError::Internal {
        message: "missing required --input".to_string(),
    })?;
    let out = out.ok_or_else(|| AppError::Internal {
        message: "missing required --out".to_string(),
    })?;

    let records = count_fastq_records(&input)?;
    let body = serde_json::to_string_pretty(&EnumerateResponse {
        ok: true,
        data: Some(EnumeratePayload {
            input: input.display().to_string(),
            records,
        }),
        error: None,
    })
    .map_err(|error| AppError::Internal {
        message: format!("failed to serialize enumerate response: {error}"),
    })?;

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    fs::write(&out, body).map_err(|error| AppError::WriteError {
        path: out.clone(),
        message: error.to_string(),
    })?;

    Ok(())
}

fn print_help() {
    println!("Usage: bamana_fastq_gz_enumerate --input <reads.fastq.gz> --out <counts.json>");
}
