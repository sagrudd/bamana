use std::path::Path;

use serde::Serialize;

use crate::{
    error::BamanaError,
    formats::probe::{Confidence, ContainerKind, DetectedFormat},
};

#[derive(Debug, Serialize)]
pub struct CommandResponse<T>
where
    T: Serialize,
{
    pub ok: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
}

impl<T> CommandResponse<T>
where
    T: Serialize,
{
    pub fn success(command: &str, path: Option<&Path>, data: T) -> Self {
        Self {
            ok: true,
            command: command.to_string(),
            path: path.map(path_to_string),
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(
        command: &str,
        path: Option<&Path>,
        data: Option<T>,
        error: ErrorResponse,
    ) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            path: path.map(path_to_string),
            data,
            error: Some(error),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl From<BamanaError> for ErrorResponse {
    fn from(error: BamanaError) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.message(),
            detail: error.detail(),
            hint: error.hint(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IdentifyData {
    pub detected_format: DetectedFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerKind>,
    pub confidence: Confidence,
}

#[derive(Debug, Serialize)]
pub struct VerifyData {
    pub detected_format: DetectedFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerKind>,
    pub is_bam: bool,
    pub shallow_verified: bool,
    pub deep_validated: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckEofData {
    pub detected_format: DetectedFormat,
    pub bgzf_eof_present: bool,
    pub complete: bool,
    pub semantic_note: String,
}

#[derive(Debug, Serialize)]
pub struct HeaderData {
    pub format: DetectedFormat,
    pub header: BamHeader,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BamHeader {
    pub hd: HeaderHd,
    pub references: Vec<ReferenceSequence>,
    pub read_groups: Vec<ReadGroup>,
    pub programs: Vec<ProgramRecord>,
    pub comments: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HeaderHd {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_sort_order: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceSequence {
    pub name: String,
    pub length: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReadGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProgramRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_line: Option<String>,
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
