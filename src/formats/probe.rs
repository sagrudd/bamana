use std::{fs, fs::File, io::Read, path::Path};

use serde::Serialize;

use crate::{bgzf, error::AppError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum DetectedFormat {
    #[serde(rename = "BAM")]
    Bam,
    #[serde(rename = "SAM")]
    Sam,
    #[serde(rename = "CRAM")]
    Cram,
    #[serde(rename = "FASTQ")]
    Fastq,
    #[serde(rename = "FASTQ.GZ")]
    FastqGz,
    #[serde(rename = "FASTA")]
    Fasta,
    #[serde(rename = "BED")]
    Bed,
    #[serde(rename = "GFF")]
    Gff,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl std::fmt::Display for DetectedFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bam => write!(f, "BAM"),
            Self::Sam => write!(f, "SAM"),
            Self::Cram => write!(f, "CRAM"),
            Self::Fastq => write!(f, "FASTQ"),
            Self::FastqGz => write!(f, "FASTQ.GZ"),
            Self::Fasta => write!(f, "FASTA"),
            Self::Bed => write!(f, "BED"),
            Self::Gff => write!(f, "GFF"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ContainerKind {
    #[serde(rename = "BGZF")]
    Bgzf,
    #[serde(rename = "GZIP")]
    Gzip,
    #[serde(rename = "PLAIN_TEXT")]
    PlainText,
    #[serde(rename = "BINARY")]
    Binary,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeResult {
    pub detected_format: DetectedFormat,
    pub container: ContainerKind,
    pub confidence: Confidence,
    pub bam_magic_present: bool,
}

impl ProbeResult {
    fn new(
        detected_format: DetectedFormat,
        container: ContainerKind,
        confidence: Confidence,
    ) -> Self {
        Self {
            detected_format,
            container,
            confidence,
            bam_magic_present: false,
        }
    }

    fn with_bam_magic(mut self, bam_magic_present: bool) -> Self {
        self.bam_magic_present = bam_magic_present;
        self
    }
}

pub fn probe_path(path: &Path) -> Result<ProbeResult, AppError> {
    let metadata = fs::metadata(path).map_err(|error| AppError::from_io(path, error))?;
    if !metadata.is_file() {
        return Err(AppError::UnsupportedFormat {
            path: path.to_path_buf(),
            format: "directories are not supported inputs".to_string(),
        });
    }

    let mut file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let mut prefix = vec![0_u8; 8 * 1024];
    let bytes_read = file
        .read(&mut prefix)
        .map_err(|error| AppError::from_io(path, error))?;
    prefix.truncate(bytes_read);

    if prefix.is_empty() {
        return Ok(extension_hint(path).unwrap_or_else(|| {
            ProbeResult::new(
                DetectedFormat::Unknown,
                ContainerKind::Unknown,
                Confidence::Low,
            )
        }));
    }

    if bgzf::is_bgzf_header(&prefix) {
        let bam_magic_present = bgzf::first_member_starts_with_bam_magic(path).unwrap_or(false);
        if bam_magic_present {
            return Ok(ProbeResult::new(
                DetectedFormat::Bam,
                ContainerKind::Bgzf,
                Confidence::High,
            )
            .with_bam_magic(true));
        }

        if path_has_extension(path, &["bam"]) {
            return Ok(ProbeResult::new(
                DetectedFormat::Bam,
                ContainerKind::Bgzf,
                Confidence::Medium,
            )
            .with_bam_magic(false));
        }

        return Ok(ProbeResult::new(
            DetectedFormat::Unknown,
            ContainerKind::Bgzf,
            Confidence::Low,
        )
        .with_bam_magic(false));
    }

    if bgzf::is_gzip_signature(&prefix) {
        if path_has_extension(path, &["bam"]) {
            return Ok(ProbeResult::new(
                DetectedFormat::Bam,
                ContainerKind::Gzip,
                Confidence::Low,
            ));
        }

        if path_has_extension(path, &["fastq.gz", "fq.gz"]) {
            return Ok(ProbeResult::new(
                DetectedFormat::FastqGz,
                ContainerKind::Gzip,
                Confidence::High,
            ));
        }

        return Ok(extension_hint(path).map_or_else(
            || {
                ProbeResult::new(
                    DetectedFormat::Unknown,
                    ContainerKind::Gzip,
                    Confidence::Low,
                )
            },
            |hint| ProbeResult {
                container: ContainerKind::Gzip,
                ..hint
            },
        ));
    }

    if prefix.starts_with(b"CRAM") {
        return Ok(ProbeResult::new(
            DetectedFormat::Cram,
            ContainerKind::Binary,
            Confidence::High,
        ));
    }

    if is_likely_text(&prefix) {
        let text = String::from_utf8_lossy(&prefix);

        if looks_like_fasta(&text) {
            return Ok(ProbeResult::new(
                DetectedFormat::Fasta,
                ContainerKind::PlainText,
                Confidence::High,
            ));
        }
        if looks_like_fastq(&text) {
            return Ok(ProbeResult::new(
                DetectedFormat::Fastq,
                ContainerKind::PlainText,
                Confidence::High,
            ));
        }
        if looks_like_sam(&text) {
            return Ok(ProbeResult::new(
                DetectedFormat::Sam,
                ContainerKind::PlainText,
                Confidence::Medium,
            ));
        }
        if looks_like_gff(&text) {
            return Ok(ProbeResult::new(
                DetectedFormat::Gff,
                ContainerKind::PlainText,
                Confidence::Medium,
            ));
        }
        if looks_like_bed(&text) {
            return Ok(ProbeResult::new(
                DetectedFormat::Bed,
                ContainerKind::PlainText,
                Confidence::Medium,
            ));
        }

        return Ok(extension_hint(path).map_or_else(
            || {
                ProbeResult::new(
                    DetectedFormat::Unknown,
                    ContainerKind::PlainText,
                    Confidence::Low,
                )
            },
            |hint| ProbeResult {
                container: ContainerKind::PlainText,
                ..hint
            },
        ));
    }

    Ok(extension_hint(path).unwrap_or_else(|| {
        ProbeResult::new(
            DetectedFormat::Unknown,
            ContainerKind::Binary,
            Confidence::Low,
        )
    }))
}

fn extension_hint(path: &Path) -> Option<ProbeResult> {
    if path_has_extension(path, &["bam"]) {
        return Some(ProbeResult::new(
            DetectedFormat::Bam,
            ContainerKind::Bgzf,
            Confidence::Medium,
        ));
    }
    if path_has_extension(path, &["sam"]) {
        return Some(ProbeResult::new(
            DetectedFormat::Sam,
            ContainerKind::PlainText,
            Confidence::Medium,
        ));
    }
    if path_has_extension(path, &["cram"]) {
        return Some(ProbeResult::new(
            DetectedFormat::Cram,
            ContainerKind::Binary,
            Confidence::Medium,
        ));
    }
    if path_has_extension(path, &["fastq.gz", "fq.gz"]) {
        return Some(ProbeResult::new(
            DetectedFormat::FastqGz,
            ContainerKind::Gzip,
            Confidence::Medium,
        ));
    }
    if path_has_extension(path, &["fastq", "fq"]) {
        return Some(ProbeResult::new(
            DetectedFormat::Fastq,
            ContainerKind::PlainText,
            Confidence::Medium,
        ));
    }
    if path_has_extension(path, &["fasta", "fa", "fna"]) {
        return Some(ProbeResult::new(
            DetectedFormat::Fasta,
            ContainerKind::PlainText,
            Confidence::Medium,
        ));
    }
    if path_has_extension(path, &["bed"]) {
        return Some(ProbeResult::new(
            DetectedFormat::Bed,
            ContainerKind::PlainText,
            Confidence::Medium,
        ));
    }
    if path_has_extension(path, &["gff", "gff3"]) {
        return Some(ProbeResult::new(
            DetectedFormat::Gff,
            ContainerKind::PlainText,
            Confidence::Medium,
        ));
    }
    None
}

fn path_has_extension(path: &Path, suffixes: &[&str]) -> bool {
    let lowered = path.to_string_lossy().to_ascii_lowercase();
    suffixes
        .iter()
        .any(|suffix| lowered.ends_with(&format!(".{suffix}")) || lowered == *suffix)
}

fn is_likely_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0)
}

fn looks_like_fasta(text: &str) -> bool {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.starts_with('>'))
}

fn looks_like_fastq(text: &str) -> bool {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    matches!(
        (lines.next(), lines.next(), lines.next(), lines.next()),
        (Some(first), Some(_second), Some(third), Some(_fourth))
            if first.starts_with('@') && third.starts_with('+')
    )
}

fn looks_like_sam(text: &str) -> bool {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    match lines.next() {
        Some(line)
            if line.starts_with("@HD")
                || line.starts_with("@SQ")
                || line.starts_with("@RG")
                || line.starts_with("@PG")
                || line.starts_with("@CO") =>
        {
            true
        }
        Some(line) => line.split('\t').count() >= 11,
        None => false,
    }
}

fn looks_like_bed(text: &str) -> bool {
    let Some(line) = text.lines().find(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("track")
    }) else {
        return false;
    };

    let fields: Vec<_> = line.split('\t').collect();
    fields.len() >= 3 && fields[1].parse::<u64>().is_ok() && fields[2].parse::<u64>().is_ok()
}

fn looks_like_gff(text: &str) -> bool {
    if text.lines().any(|line| line.starts_with("##gff-version")) {
        return true;
    }

    let Some(line) = text.lines().find(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    }) else {
        return false;
    };

    line.split('\t').count() == 9
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Confidence, ContainerKind, DetectedFormat, probe_path};
    use crate::formats::bgzf::test_support;

    #[test]
    fn identifies_bgzf_bam_with_high_confidence() {
        let path =
            test_support::write_temp_file("identify-bam", "bam", &test_support::build_bam_file());
        let result = probe_path(&path).expect("probe should succeed");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(result.detected_format, DetectedFormat::Bam);
        assert_eq!(result.container, ContainerKind::Bgzf);
        assert_eq!(result.confidence, Confidence::High);
        assert!(result.bam_magic_present);
    }

    #[test]
    fn identifies_fasta_text() {
        let path = test_support::write_temp_file("identify-fasta", "fa", b">chr1\nACGT\n");
        let result = probe_path(&path).expect("probe should succeed");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(result.detected_format, DetectedFormat::Fasta);
        assert_eq!(result.container, ContainerKind::PlainText);
    }

    #[test]
    fn falls_back_to_unknown_for_unclassified_binary() {
        let path = test_support::write_temp_file("identify-binary", "bin", &[0, 1, 2, 3, 4]);
        let result = probe_path(&path).expect("probe should succeed");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(result.detected_format, DetectedFormat::Unknown);
    }
}
