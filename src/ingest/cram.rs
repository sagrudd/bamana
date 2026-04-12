use std::{
    collections::hash_map::DefaultHasher,
    fmt, fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use noodles_bam as bam;
use noodles_cram as cram;
use noodles_fasta::{self as fasta, repository::adapters::IndexedReader};
use noodles_sam::alignment::io::Write as _;
use serde::Serialize;

use crate::{
    bam::{
        header::{ReferenceRecord, parse_bam_header_from_reader},
        reader::BamReader,
        records::{RecordLayout, read_next_record_layout},
    },
    error::AppError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ConsumeReferencePolicy {
    Strict,
    AllowEmbedded,
    AllowCache,
    AutoConservative,
}

impl fmt::Display for ConsumeReferencePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strict => write!(f, "strict"),
            Self::AllowEmbedded => write!(f, "allow-embedded"),
            Self::AllowCache => write!(f, "allow-cache"),
            Self::AutoConservative => write!(f, "auto-conservative"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumeReferenceSourceUsed {
    ExplicitFasta,
    EmbeddedOrNotRequired,
}

#[derive(Debug, Clone)]
pub struct CramReferenceContext {
    pub policy: ConsumeReferencePolicy,
    pub explicit_reference_provided: bool,
    pub reference_cache_provided: bool,
    pub source_used_hint: Option<ConsumeReferenceSourceUsed>,
    pub decode_without_external_reference_hint: Option<bool>,
    pub notes: Vec<String>,
    plan: CramReferencePlan,
}

#[derive(Debug, Clone)]
enum CramReferencePlan {
    ExplicitFasta(PathBuf),
    NoExternalReferenceAttempt,
}

#[derive(Debug)]
pub struct CramNormalization {
    pub raw_header_text: String,
    pub references: Vec<ReferenceRecord>,
    pub records: Vec<RecordLayout>,
    pub source_used: ConsumeReferenceSourceUsed,
    pub decode_without_external_reference: bool,
}

pub fn prepare_reference_context(
    command_path: &Path,
    policy: ConsumeReferencePolicy,
    reference: Option<&Path>,
    reference_cache: Option<&Path>,
    dry_run: bool,
) -> Result<CramReferenceContext, AppError> {
    let explicit_reference_provided = reference.is_some();
    let reference_cache_provided = reference_cache.is_some();
    let mut notes = Vec::new();

    let plan = if let Some(reference_path) = reference {
        let validated_reference = validate_reference_fasta(reference_path)?;
        if reference_cache_provided {
            notes.push(
                "An explicit reference FASTA was provided and takes precedence over the reference cache path in this slice."
                    .to_string(),
            );
        }
        CramReferencePlan::ExplicitFasta(validated_reference)
    } else {
        match policy {
            ConsumeReferencePolicy::Strict => {
                return Err(AppError::ReferenceRequired {
                    path: command_path.to_path_buf(),
                    detail: "CRAM ingestion under the strict policy currently requires an explicit indexed reference FASTA supplied via --reference."
                        .to_string(),
                });
            }
            ConsumeReferencePolicy::AllowEmbedded => {
                if dry_run {
                    notes.push(
                        "Dry-run validated the allow-embedded policy shape but cannot prove decode success without executing the CRAM reader."
                            .to_string(),
                    );
                }
                CramReferencePlan::NoExternalReferenceAttempt
            }
            ConsumeReferencePolicy::AllowCache => {
                let detail = if let Some(cache_path) = reference_cache {
                    format!(
                        "Cache-based CRAM decoding from {} is planned but not implemented in the current Rust slice.",
                        cache_path.to_string_lossy()
                    )
                } else {
                    "The allow-cache policy requires an explicit --reference-cache path, and cache-backed CRAM decoding is not implemented in the current Rust slice."
                        .to_string()
                };

                return Err(AppError::Unimplemented {
                    path: command_path.to_path_buf(),
                    detail,
                });
            }
            ConsumeReferencePolicy::AutoConservative => {
                if let Some(cache_path) = reference_cache {
                    return Err(AppError::Unimplemented {
                        path: command_path.to_path_buf(),
                        detail: format!(
                            "Cache-backed CRAM decoding from {} is planned but not implemented in the current Rust slice.",
                            cache_path.to_string_lossy()
                        ),
                    });
                }

                if dry_run {
                    notes.push(
                        "Dry-run validated the auto-conservative policy shape but cannot prove decode success without executing the CRAM reader."
                            .to_string(),
                    );
                }
                CramReferencePlan::NoExternalReferenceAttempt
            }
        }
    };

    let (source_used_hint, decode_without_external_reference_hint) = match plan {
        CramReferencePlan::ExplicitFasta(_) => {
            (Some(ConsumeReferenceSourceUsed::ExplicitFasta), Some(false))
        }
        CramReferencePlan::NoExternalReferenceAttempt => (None, None),
    };

    Ok(CramReferenceContext {
        policy,
        explicit_reference_provided,
        reference_cache_provided,
        source_used_hint,
        decode_without_external_reference_hint,
        notes,
        plan,
    })
}

pub fn normalize_cram_to_record_layouts(
    input_path: &Path,
    context: &CramReferenceContext,
) -> Result<CramNormalization, AppError> {
    let mut builder = cram::io::reader::Builder::default();

    let source_used = match &context.plan {
        CramReferencePlan::ExplicitFasta(reference_path) => {
            let repository = build_reference_repository(reference_path)?;
            builder = builder.set_reference_sequence_repository(repository);
            ConsumeReferenceSourceUsed::ExplicitFasta
        }
        CramReferencePlan::NoExternalReferenceAttempt => {
            ConsumeReferenceSourceUsed::EmbeddedOrNotRequired
        }
    };

    let mut reader = builder
        .build_from_path(input_path)
        .map_err(|error| AppError::from_io(input_path, error))?;
    let header = reader
        .read_header()
        .map_err(|error| map_cram_decode_error(input_path, context, error))?;

    let temp_bam_path = temporary_normalized_bam_path(input_path);
    if temp_bam_path.exists() {
        let _ = fs::remove_file(&temp_bam_path);
    }

    let write_result = (|| -> Result<(), AppError> {
        let mut writer = bam::io::writer::Builder::default()
            .build_from_path(&temp_bam_path)
            .map_err(|error| AppError::WriteError {
                path: temp_bam_path.clone(),
                message: error.to_string(),
            })?;

        writer
            .write_header(&header)
            .map_err(|error| AppError::CramDecodeFailed {
                path: input_path.to_path_buf(),
                detail: format!(
                    "CRAM header was decoded but temporary BAM header emission failed: {error}"
                ),
            })?;

        for result in reader.records(&header) {
            let record =
                result.map_err(|error| map_cram_decode_error(input_path, context, error))?;
            writer
                .write_alignment_record(&header, &record)
                .map_err(|error| AppError::CramDecodeFailed {
                    path: input_path.to_path_buf(),
                    detail: format!(
                        "CRAM record decoding succeeded but BAM normalization failed: {error}"
                    ),
                })?;
        }

        writer.try_finish().map_err(|error| AppError::WriteError {
            path: temp_bam_path.clone(),
            message: error.to_string(),
        })?;

        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_bam_path);
        return Err(error);
    }

    let parsed = (|| -> Result<CramNormalization, AppError> {
        let mut bam_reader = BamReader::open(&temp_bam_path)?;
        let header_payload = parse_bam_header_from_reader(&mut bam_reader)?;
        let mut records = Vec::new();

        while let Some(layout) = read_next_record_layout(&mut bam_reader)? {
            records.push(layout);
        }

        Ok(CramNormalization {
            raw_header_text: header_payload.header.raw_header_text,
            references: header_payload.header.references,
            records,
            source_used,
            decode_without_external_reference: matches!(
                source_used,
                ConsumeReferenceSourceUsed::EmbeddedOrNotRequired
            ),
        })
    })();

    let _ = fs::remove_file(&temp_bam_path);
    parsed
}

fn validate_reference_fasta(reference_path: &Path) -> Result<PathBuf, AppError> {
    if !reference_path.exists() {
        return Err(AppError::ReferenceNotFound {
            path: reference_path.to_path_buf(),
            detail: "The explicit reference FASTA path does not exist.".to_string(),
        });
    }

    let fai_path = PathBuf::from(format!("{}.fai", reference_path.to_string_lossy()));
    if !fai_path.exists() {
        return Err(AppError::ReferenceNotFound {
            path: reference_path.to_path_buf(),
            detail: "Stage 2 CRAM ingestion currently requires an indexed FASTA with an adjacent .fai file."
                .to_string(),
        });
    }

    Ok(reference_path.to_path_buf())
}

fn build_reference_repository(reference_path: &Path) -> Result<fasta::Repository, AppError> {
    let indexed_reader = fasta::io::indexed_reader::Builder::default()
        .build_from_path(reference_path)
        .map_err(|error| AppError::ReferenceNotFound {
            path: reference_path.to_path_buf(),
            detail: format!(
                "The explicit reference FASTA could not be opened as an indexed FASTA: {error}"
            ),
        })?;

    let adapter = IndexedReader::new(indexed_reader);
    Ok(fasta::Repository::new(adapter))
}

fn map_cram_decode_error(
    input_path: &Path,
    context: &CramReferenceContext,
    error: io::Error,
) -> AppError {
    let detail = error.to_string();

    if matches!(context.plan, CramReferencePlan::NoExternalReferenceAttempt)
        && looks_like_reference_requirement(&detail)
    {
        return AppError::ReferenceRequired {
            path: input_path.to_path_buf(),
            detail: format!(
                "CRAM decoding under the {} policy could not proceed without explicit reference material: {detail}",
                context.policy
            ),
        };
    }

    AppError::CramDecodeFailed {
        path: input_path.to_path_buf(),
        detail,
    }
}

fn looks_like_reference_requirement(detail: &str) -> bool {
    let normalized = detail.to_ascii_lowercase();
    normalized.contains("reference")
        || normalized.contains("md5")
        || normalized.contains("repository")
        || normalized.contains("sequence not found")
        || normalized.contains("missing sequence")
}

fn temporary_normalized_bam_path(input_path: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    input_path.hash(&mut hasher);
    let suffix = hasher.finish();
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("input.cram");

    std::env::temp_dir().join(format!(
        ".bamana-consume-cram-{}-{suffix}.bam",
        file_name.replace('/', "_")
    ))
}

#[cfg(test)]
mod tests {
    use super::{ConsumeReferencePolicy, prepare_reference_context};

    #[test]
    fn strict_policy_requires_explicit_reference() {
        let error = prepare_reference_context(
            std::path::Path::new("out.bam"),
            ConsumeReferencePolicy::Strict,
            None,
            None,
            true,
        )
        .expect_err("strict policy should require an explicit reference");

        assert_eq!(error.to_json_error().code, "reference_required");
    }

    #[test]
    fn auto_conservative_dry_run_without_reference_is_allowed() {
        let context = prepare_reference_context(
            std::path::Path::new("out.bam"),
            ConsumeReferencePolicy::AutoConservative,
            None,
            None,
            true,
        )
        .expect("auto-conservative should allow a conservative dry-run without explicit reference");

        assert!(context.notes.iter().any(|note| note.contains("Dry-run")));
        assert!(context.source_used_hint.is_none());
    }
}
