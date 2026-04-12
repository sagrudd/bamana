use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{formats::probe::DetectedFormat, json::JsonError};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("file not found: {path}")]
    FileNotFound { path: PathBuf },
    #[error("permission denied: {path}")]
    PermissionDenied { path: PathBuf },
    #[error("i/o error for {path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("unable to determine file format: {path}")]
    UnknownFormat { path: PathBuf },
    #[error("input is not BAM: {path}")]
    NotBam {
        path: PathBuf,
        detected_format: DetectedFormat,
    },
    #[error("input does not satisfy shallow BAM expectations: {path}")]
    InvalidBam { path: PathBuf, detail: String },
    #[error("bam header could not be parsed: {path}")]
    InvalidHeader { path: PathBuf, detail: String },
    #[error("replacement header could not be parsed: {path}")]
    InvalidHeaderFile { path: PathBuf, detail: String },
    #[error("requested header mutation is invalid: {path}")]
    InvalidHeaderMutation { path: PathBuf, detail: String },
    #[error("requested RG annotation is invalid: {path}")]
    InvalidRgRequest { path: PathBuf, detail: String },
    #[error("requested deduplicate mode is invalid: {path}")]
    InvalidDeduplicateMode { path: PathBuf, detail: String },
    #[error("bam record could not be parsed: {path}")]
    InvalidRecord { path: PathBuf, detail: String },
    #[error("bam index could not be parsed: {path}")]
    InvalidIndex { path: PathBuf, detail: String },
    #[error("index format is not supported: {path}")]
    UnsupportedIndex { path: PathBuf, detail: String },
    #[error("no usable bam index was found: {path}")]
    MissingIndex {
        path: PathBuf,
        detail: Option<String>,
    },
    #[error("input bam headers are not compatible: {path}")]
    IncompatibleHeaders { path: PathBuf, detail: String },
    #[error("invalid merge request: {path}")]
    InvalidMergeRequest { path: PathBuf, detail: String },
    #[error("invalid consume request: {path}")]
    InvalidConsumeRequest { path: PathBuf, detail: String },
    #[error("mixed input modes are not allowed: {path}")]
    MixedInputModesNotAllowed { path: PathBuf, detail: String },
    #[error("input format is not supported for consume: {path}")]
    UnsupportedInputFormat { path: PathBuf, format: String },
    #[error("directory entry is not supported for consume: {path}")]
    UnsupportedDirectoryEntry { path: PathBuf, detail: String },
    #[error("reference material is required for this consume mode: {path}")]
    ReferenceRequired { path: PathBuf, detail: String },
    #[error("reference material could not be resolved: {path}")]
    ReferenceNotFound { path: PathBuf, detail: String },
    #[error("input is not supported for the selected consume mode: {path}")]
    UnsupportedInputForMode { path: PathBuf, detail: String },
    #[error("input is not supported for this command: {path}")]
    UnsupportedInputForCommand { path: PathBuf, detail: String },
    #[error("cram input could not be decoded: {path}")]
    CramDecodeFailed { path: PathBuf, detail: String },
    #[error("fastq input could not be parsed: {path}")]
    InvalidFastq { path: PathBuf, detail: String },
    #[error("requested duplication identity mode is invalid: {path}")]
    InvalidIdentityMode { path: PathBuf, detail: String },
    #[error("requested read group is missing: {path}")]
    MissingReadGroup { path: PathBuf, id: String },
    #[error("requested read group would be duplicated: {path}")]
    DuplicateReadGroup { path: PathBuf, id: String },
    #[error("existing RG tags conflict with the requested assignment: {path}")]
    ConflictingReadGroupTags { path: PathBuf, detail: String },
    #[error("true in-place reheader is not feasible: {path}")]
    InPlaceNotFeasible { path: PathBuf, detail: String },
    #[error("requested reheader execution mode is not supported: {path}")]
    UnsupportedReheaderMode { path: PathBuf, detail: String },
    #[error("refusing to overwrite existing output: {path}")]
    OutputExists { path: PathBuf },
    #[error("failed to write output: {path}")]
    WriteError { path: PathBuf, message: String },
    #[error("functionality is not implemented for this input: {path}")]
    Unimplemented { path: PathBuf, detail: String },
    #[error("bam validation failed: {path}")]
    ValidationFailed { path: PathBuf, detail: String },
    #[error("requested bam tag is invalid: {path}")]
    InvalidTag { path: PathBuf, tag: String },
    #[error("requested bam aux type is invalid: {path}")]
    InvalidTagType { path: PathBuf, tag_type: String },
    #[error("mapping state could not be determined reliably: {path}")]
    ParseUncertainty { path: PathBuf, detail: String },
    #[error("input parsing failed: {path}")]
    ParseError { path: PathBuf, detail: String },
    #[error("bam checksum could not be computed reliably: {path}")]
    ChecksumUncertainty { path: PathBuf, detail: String },
    #[error("checksum verification failed: {path}")]
    ChecksumMismatch { path: PathBuf, detail: String },
    #[error("bam auxiliary fields could not be parsed reliably: {path}")]
    TagParseUncertainty { path: PathBuf, detail: String },
    #[error("bam summary could not be generated reliably: {path}")]
    SummaryUncertainty { path: PathBuf, detail: String },
    #[error("file is truncated or incomplete: {path}")]
    TruncatedFile { path: PathBuf, detail: String },
    #[error("unsupported format for this command: {path}")]
    UnsupportedFormat { path: PathBuf, format: String },
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl AppError {
    pub fn to_json_error(&self) -> JsonError {
        JsonError {
            code: self.code().to_string(),
            message: self.message(),
            detail: self.detail(),
            hint: self.hint(),
        }
    }

    pub fn from_io(path: &Path, error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::FileNotFound {
                path: path.to_path_buf(),
            },
            io::ErrorKind::PermissionDenied => Self::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => Self::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::FileNotFound { .. } => "file_not_found",
            Self::PermissionDenied { .. } => "permission_denied",
            Self::Io { .. } => "io_error",
            Self::UnknownFormat { .. } => "unknown_format",
            Self::NotBam { .. } => "not_bam",
            Self::InvalidBam { .. } => "invalid_bam",
            Self::InvalidHeader { .. } => "invalid_header",
            Self::InvalidHeaderFile { .. } => "invalid_header_file",
            Self::InvalidHeaderMutation { .. } => "invalid_header_mutation",
            Self::InvalidRgRequest { .. } => "invalid_rg_request",
            Self::InvalidDeduplicateMode { .. } => "invalid_deduplicate_mode",
            Self::InvalidRecord { .. } => "invalid_record",
            Self::InvalidIndex { .. } => "invalid_index",
            Self::UnsupportedIndex { .. } => "unsupported_index",
            Self::MissingIndex { .. } => "missing_index",
            Self::IncompatibleHeaders { .. } => "incompatible_headers",
            Self::InvalidMergeRequest { .. } => "invalid_merge_mode",
            Self::InvalidConsumeRequest { .. } => "invalid_consume_mode",
            Self::MixedInputModesNotAllowed { .. } => "mixed_input_modes_not_allowed",
            Self::UnsupportedInputFormat { .. } => "unsupported_input_format",
            Self::UnsupportedDirectoryEntry { .. } => "unsupported_directory_entry",
            Self::ReferenceRequired { .. } => "reference_required",
            Self::ReferenceNotFound { .. } => "reference_not_found",
            Self::UnsupportedInputForMode { .. } => "unsupported_input_for_mode",
            Self::UnsupportedInputForCommand { .. } => "unsupported_input_for_command",
            Self::CramDecodeFailed { .. } => "cram_decode_failed",
            Self::InvalidFastq { .. } => "invalid_fastq",
            Self::InvalidIdentityMode { .. } => "invalid_identity_mode",
            Self::MissingReadGroup { .. } => "missing_read_group",
            Self::DuplicateReadGroup { .. } => "duplicate_read_group",
            Self::ConflictingReadGroupTags { .. } => "conflicting_read_group_tags",
            Self::InPlaceNotFeasible { .. } => "in_place_not_feasible",
            Self::UnsupportedReheaderMode { .. } => "unsupported_reheader_mode",
            Self::OutputExists { .. } => "output_exists",
            Self::WriteError { .. } => "write_error",
            Self::Unimplemented { .. } => "unimplemented",
            Self::ValidationFailed { .. } => "invalid_bam",
            Self::InvalidTag { .. } => "invalid_tag",
            Self::InvalidTagType { .. } => "invalid_tag_type",
            Self::ParseUncertainty { .. } => "parse_uncertainty",
            Self::ParseError { .. } => "parse_error",
            Self::ChecksumUncertainty { .. } => "parse_uncertainty",
            Self::ChecksumMismatch { .. } => "checksum_mismatch",
            Self::TagParseUncertainty { .. } => "parse_uncertainty",
            Self::SummaryUncertainty { .. } => "parse_uncertainty",
            Self::TruncatedFile { .. } => "truncated_file",
            Self::UnsupportedFormat { .. } => "unsupported_format",
            Self::Internal { .. } => "internal_error",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::FileNotFound { .. } => "Input file was not found.".to_string(),
            Self::PermissionDenied { .. } => {
                "Permission denied while opening the input file.".to_string()
            }
            Self::Io { .. } => "An I/O error occurred while reading the input file.".to_string(),
            Self::UnknownFormat { .. } => "Unable to determine file format.".to_string(),
            Self::NotBam { .. } => "Input is not a BAM file.".to_string(),
            Self::InvalidBam { .. } => {
                "Input does not satisfy shallow BAM verification.".to_string()
            }
            Self::InvalidHeader { .. } => "BAM header could not be parsed.".to_string(),
            Self::InvalidHeaderFile { .. } => {
                "Replacement header file could not be parsed.".to_string()
            }
            Self::InvalidHeaderMutation { .. } => {
                "Requested BAM header mutation is not valid.".to_string()
            }
            Self::InvalidRgRequest { .. } => {
                "Requested BAM read-group annotation options are not valid.".to_string()
            }
            Self::InvalidDeduplicateMode { .. } => {
                "Requested deduplicate mode or policy is not valid.".to_string()
            }
            Self::InvalidRecord { .. } => "BAM record could not be parsed.".to_string(),
            Self::InvalidIndex { .. } => "BAM index could not be parsed.".to_string(),
            Self::UnsupportedIndex { .. } => "Index format is not supported.".to_string(),
            Self::MissingIndex { .. } => "No usable BAM index was found.".to_string(),
            Self::IncompatibleHeaders { .. } => {
                "Input alignment headers are not compatible for a combined BAM output."
                    .to_string()
            }
            Self::InvalidMergeRequest { .. } => "Invalid BAM merge options.".to_string(),
            Self::InvalidConsumeRequest { .. } => "Invalid consume options.".to_string(),
            Self::MixedInputModesNotAllowed { .. } => {
                "Mixed raw-read and alignment-bearing inputs are not allowed in the selected consume mode.".to_string()
            }
            Self::UnsupportedInputFormat { .. } => {
                "Input format is not supported for consume.".to_string()
            }
            Self::UnsupportedDirectoryEntry { .. } => {
                "Directory entry is not supported for consume.".to_string()
            }
            Self::ReferenceRequired { .. } => {
                "CRAM ingestion requires reference material under the selected policy.".to_string()
            }
            Self::ReferenceNotFound { .. } => {
                "Reference material could not be resolved.".to_string()
            }
            Self::UnsupportedInputForMode { .. } => {
                "Detected input is not supported in the selected consume mode.".to_string()
            }
            Self::UnsupportedInputForCommand { .. } => {
                "Detected input is not supported by this command.".to_string()
            }
            Self::CramDecodeFailed { .. } => "CRAM input could not be decoded.".to_string(),
            Self::InvalidFastq { .. } => "FASTQ input could not be parsed.".to_string(),
            Self::InvalidIdentityMode { .. } => {
                "Requested duplication identity mode is not valid for this input.".to_string()
            }
            Self::MissingReadGroup { .. } => {
                "The requested read group could not be updated because it does not exist in the BAM header.".to_string()
            }
            Self::DuplicateReadGroup { .. } => {
                "The requested read group change would create a duplicate @RG ID.".to_string()
            }
            Self::ConflictingReadGroupTags { .. } => {
                "Existing RG tags conflict with the requested read-group assignment."
                    .to_string()
            }
            Self::InPlaceNotFeasible { .. } => {
                "The requested true in-place reheader operation could not be proven safe."
                    .to_string()
            }
            Self::UnsupportedReheaderMode { .. } => {
                "The requested reheader execution mode is not supported in this slice."
                    .to_string()
            }
            Self::OutputExists { .. } => {
                "Output path already exists and overwrite was not requested.".to_string()
            }
            Self::WriteError { .. } => "Failed to write output.".to_string(),
            Self::Unimplemented { .. } => {
                "This functionality is not implemented in this slice.".to_string()
            }
            Self::ValidationFailed { .. } => "BAM validation failed.".to_string(),
            Self::InvalidTag { .. } => "Requested BAM tag is invalid.".to_string(),
            Self::InvalidTagType { .. } => "Requested BAM auxiliary type is invalid.".to_string(),
            Self::ParseUncertainty { .. } => {
                "The requested assessment could not be completed reliably from the available evidence."
                    .to_string()
            }
            Self::ParseError { .. } => "Input parsing failed.".to_string(),
            Self::ChecksumUncertainty { .. } => {
                "BAM checksum could not be computed reliably.".to_string()
            }
            Self::ChecksumMismatch { .. } => "Checksum verification failed.".to_string(),
            Self::TagParseUncertainty { .. } => {
                "BAM auxiliary fields could not be parsed reliably.".to_string()
            }
            Self::SummaryUncertainty { .. } => {
                "BAM summary could not be generated reliably.".to_string()
            }
            Self::TruncatedFile { .. } => "File is truncated or incomplete.".to_string(),
            Self::UnsupportedFormat { .. } => {
                "Detected format is not supported by this command.".to_string()
            }
            Self::Internal { .. } => "An internal error occurred.".to_string(),
        }
    }

    fn detail(&self) -> Option<String> {
        match self {
            Self::Io { message, .. } => Some(message.clone()),
            Self::NotBam {
                detected_format, ..
            } => Some(format!("Detected format: {detected_format}.")),
            Self::InvalidBam { detail, .. } => Some(detail.clone()),
            Self::InvalidHeader { detail, .. } => Some(detail.clone()),
            Self::InvalidHeaderFile { detail, .. } => Some(detail.clone()),
            Self::InvalidHeaderMutation { detail, .. } => Some(detail.clone()),
            Self::InvalidRgRequest { detail, .. } => Some(detail.clone()),
            Self::InvalidDeduplicateMode { detail, .. } => Some(detail.clone()),
            Self::InvalidRecord { detail, .. } => Some(detail.clone()),
            Self::InvalidIndex { detail, .. } => Some(detail.clone()),
            Self::UnsupportedIndex { detail, .. } => Some(detail.clone()),
            Self::MissingIndex { detail, .. } => detail.clone(),
            Self::IncompatibleHeaders { detail, .. } => Some(detail.clone()),
            Self::InvalidMergeRequest { detail, .. } => Some(detail.clone()),
            Self::InvalidConsumeRequest { detail, .. } => Some(detail.clone()),
            Self::MixedInputModesNotAllowed { detail, .. } => Some(detail.clone()),
            Self::UnsupportedInputFormat { format, .. } => Some(format.clone()),
            Self::UnsupportedDirectoryEntry { detail, .. } => Some(detail.clone()),
            Self::ReferenceRequired { detail, .. } => Some(detail.clone()),
            Self::ReferenceNotFound { detail, .. } => Some(detail.clone()),
            Self::UnsupportedInputForMode { detail, .. } => Some(detail.clone()),
            Self::UnsupportedInputForCommand { detail, .. } => Some(detail.clone()),
            Self::CramDecodeFailed { detail, .. } => Some(detail.clone()),
            Self::InvalidFastq { detail, .. } => Some(detail.clone()),
            Self::InvalidIdentityMode { detail, .. } => Some(detail.clone()),
            Self::MissingReadGroup { id, .. } => {
                Some(format!("No @RG record with ID={id} was found."))
            }
            Self::DuplicateReadGroup { id, .. } => Some(format!(
                "Header already contains an @RG record with ID={id}."
            )),
            Self::ConflictingReadGroupTags { detail, .. } => Some(detail.clone()),
            Self::InPlaceNotFeasible { detail, .. } => Some(detail.clone()),
            Self::UnsupportedReheaderMode { detail, .. } => Some(detail.clone()),
            Self::WriteError { message, .. } => Some(message.clone()),
            Self::Unimplemented { detail, .. } => Some(detail.clone()),
            Self::ValidationFailed { detail, .. } => Some(detail.clone()),
            Self::InvalidTag { tag, .. } => Some(format!("Requested tag: {tag}.")),
            Self::InvalidTagType { tag_type, .. } => Some(format!("Requested type: {tag_type}.")),
            Self::ParseUncertainty { detail, .. } => Some(detail.clone()),
            Self::ParseError { detail, .. } => Some(detail.clone()),
            Self::ChecksumUncertainty { detail, .. } => Some(detail.clone()),
            Self::ChecksumMismatch { detail, .. } => Some(detail.clone()),
            Self::TagParseUncertainty { detail, .. } => Some(detail.clone()),
            Self::SummaryUncertainty { detail, .. } => Some(detail.clone()),
            Self::TruncatedFile { detail, .. } => Some(detail.clone()),
            Self::UnsupportedFormat { format, .. } => Some(format.clone()),
            Self::Internal { message } => Some(message.clone()),
            _ => None,
        }
    }

    fn hint(&self) -> Option<String> {
        match self {
            Self::FileNotFound { .. } => Some("Check the path and rerun the command.".to_string()),
            Self::PermissionDenied { .. } => {
                Some("Ensure the input file is readable by the current user.".to_string())
            }
            Self::Io { .. } => {
                Some("Retry the operation and confirm the file is readable.".to_string())
            }
            Self::UnknownFormat { .. } => {
                Some("Inspect the file manually or provide a supported input.".to_string())
            }
            Self::NotBam { .. } => Some(
                "Run bamana identify on the input or provide a BGZF-compressed BAM file."
                    .to_string(),
            ),
            Self::InvalidBam { .. } => {
                Some("Confirm the file is BGZF-compressed BAM and rerun bamana verify.".to_string())
            }
            Self::InvalidHeader { .. } => Some(
                "Run bamana verify to perform shallow BAM checks before parsing the header."
                    .to_string(),
            ),
            Self::InvalidHeaderFile { .. } => Some(
                "Ensure the replacement file contains only SAM-style header lines and preserves the BAM reference dictionary."
                    .to_string(),
            ),
            Self::InvalidHeaderMutation { .. } => Some(
                "Adjust the requested mutation so it preserves a structurally valid BAM header."
                    .to_string(),
            ),
            Self::InvalidRgRequest { .. } => Some(
                "Choose exactly one record-annotation mode, and use explicit header-policy flags when the target @RG may be absent."
                    .to_string(),
            ),
            Self::InvalidDeduplicateMode { .. } => Some(
                "Use a supported deduplicate mode such as contiguous-block or whole-file-append, and provide a distinct output path for applied remediation."
                    .to_string(),
            ),
            Self::InvalidRecord { .. } => Some(
                "The BAM stream could not be sampled safely; rerun bamana verify and inspect the file for truncation or corruption."
                    .to_string(),
            ),
            Self::InvalidIndex { .. } => Some(
                "Use scan mode or regenerate the BAM index before retrying this command."
                    .to_string(),
            ),
            Self::UnsupportedIndex { .. } => Some(
                "Provide a BAI index or rerun the command without relying on index-derived mapping counts."
                    .to_string(),
            ),
            Self::MissingIndex { .. } => {
                Some("Run bamana index --bam <file> or place a usable companion index next to the BAM.".to_string())
            }
            Self::IncompatibleHeaders { .. } => Some(
                "Ensure all BAM/SAM/CRAM inputs use the same reference dictionary before merging or alignment-mode consume."
                    .to_string(),
            ),
            Self::InvalidMergeRequest { .. } => Some(
                "Use --sort as shorthand for --order coordinate, and only specify --queryname-suborder with --order queryname."
                    .to_string(),
            ),
            Self::InvalidConsumeRequest { .. } => Some(
                "Use --mode alignment for BAM/SAM, --mode unmapped for FASTQ/FASTQ.GZ, and --dry-run to inspect mixed or directory-heavy requests first."
                    .to_string(),
            ),
            Self::MixedInputModesNotAllowed { .. } => Some(
                "Run separate consume operations or use an explicitly supported mixed-ingestion mode."
                    .to_string(),
            ),
            Self::UnsupportedInputFormat { .. } => Some(
                "Restrict the request to BAM, SAM, FASTQ, or FASTQ.GZ inputs in the supported consume mode."
                    .to_string(),
            ),
            Self::UnsupportedDirectoryEntry { .. } => Some(
                "Restrict directory inputs to regular files or rerun with a layout that excludes unsupported entries."
                    .to_string(),
            ),
            Self::ReferenceRequired { .. } => Some(
                "Provide the required reference material or choose a consume mode that does not require reference-backed decoding."
                    .to_string(),
            ),
            Self::ReferenceNotFound { .. } => Some(
                "Provide a readable indexed FASTA reference with an adjacent .fai file."
                    .to_string(),
            ),
            Self::UnsupportedInputForMode { .. } => Some(
                "Use --mode alignment for BAM, SAM, or CRAM inputs, and --mode unmapped for FASTQ or FASTQ.GZ inputs."
                    .to_string(),
            ),
            Self::UnsupportedInputForCommand { .. } => Some(
                "Use BAM, FASTQ, or FASTQ.GZ with this command, or choose a command that supports the detected format."
                    .to_string(),
            ),
            Self::CramDecodeFailed { .. } => Some(
                "Retry with an explicit --reference FASTA or inspect the CRAM with a validator before ingesting it."
                    .to_string(),
            ),
            Self::InvalidFastq { .. } => Some(
                "Inspect the FASTQ structure and ensure every record has valid header, plus, sequence, and quality lines."
                    .to_string(),
            ),
            Self::InvalidIdentityMode { .. } => Some(
                "Use qname_seq or qname_seq_qual for FASTQ input, and reserve qname_seq_qual_rg for BAM when read-group evidence is required."
                    .to_string(),
            ),
            Self::MissingReadGroup { .. } => Some(
                "Use --add-rg, --create-header-rg, or specify an existing RG ID."
                    .to_string(),
            ),
            Self::DuplicateReadGroup { .. } => Some(
                "Use --set-rg instead of adding a duplicate read group ID.".to_string(),
            ),
            Self::ConflictingReadGroupTags { .. } => Some(
                "Use --replace-existing to normalize all records, or inspect the BAM before retrying."
                    .to_string(),
            ),
            Self::InPlaceNotFeasible { .. } => Some(
                "Use --rewrite-minimized or --safe-rewrite if a non-in-place rewrite is acceptable."
                    .to_string(),
            ),
            Self::UnsupportedReheaderMode { .. } => Some(
                "Choose --rewrite-minimized or --safe-rewrite, or provide an explicit --out path."
                    .to_string(),
            ),
            Self::OutputExists { .. } => {
                Some("Rerun with --force to overwrite the existing output path.".to_string())
            }
            Self::WriteError { .. } => Some(
                "Check the output path, available disk space, and filesystem permissions, then retry the operation."
                    .to_string(),
            ),
            Self::Unimplemented { .. } => Some(
                "Use --dry-run to inspect the request now, or extend the current slice with the deferred functionality."
                    .to_string(),
            ),
            Self::ValidationFailed { .. } => Some(
                "Run bamana verify and bamana check_eof to distinguish shallow truncation from deeper record corruption."
                    .to_string(),
            ),
            Self::InvalidTag { .. } => Some(
                "Provide exactly two printable ASCII characters, for example NM or RG."
                    .to_string(),
            ),
            Self::InvalidTagType { .. } => Some(
                "Use one supported BAM aux type code such as A, i, f, Z, H, or B."
                    .to_string(),
            ),
            Self::ParseUncertainty { .. } => Some(
                "Run bamana verify and, when available, bamana validate.".to_string(),
            ),
            Self::ParseError { .. } => Some(
                "Verify the input format and structure before retrying the command.".to_string(),
            ),
            Self::ChecksumUncertainty { .. } => Some(
                "Run bamana validate to determine whether the BAM is structurally invalid.".to_string(),
            ),
            Self::ChecksumMismatch { .. } => Some(
                "Inspect the input and output with bamana checksum and bamana validate before relying on the transformed BAM."
                    .to_string(),
            ),
            Self::TagParseUncertainty { .. } => Some(
                "Run bamana verify and, when available, bamana validate.".to_string(),
            ),
            Self::SummaryUncertainty { .. } => Some(
                "Run bamana verify and, when available, bamana validate.".to_string(),
            ),
            Self::TruncatedFile { .. } => {
                Some("Re-transfer or regenerate the BAM file, then rerun the command.".to_string())
            }
            Self::UnsupportedFormat { .. } => {
                Some("Use a command intended for the detected format.".to_string())
            }
            Self::Internal { .. } => {
                Some("Inspect logs or rerun the command with the same input.".to_string())
            }
        }
    }
}
