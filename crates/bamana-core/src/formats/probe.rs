use std::{fs::File, io::Read, path::Path};

use serde::Serialize;

use crate::{
    error::BamanaError,
    formats::{bam, bgzf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum DetectedFormat {
    BAM,
    SAM,
    CRAM,
    FASTQ,
    #[serde(rename = "FASTQ.GZ")]
    FastqGz,
    FASTA,
    BED,
    GFF,
    UNKNOWN,
}

impl std::fmt::Display for DetectedFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BAM => write!(f, "BAM"),
            Self::SAM => write!(f, "SAM"),
            Self::CRAM => write!(f, "CRAM"),
            Self::FASTQ => write!(f, "FASTQ"),
            Self::FastqGz => write!(f, "FASTQ.GZ"),
            Self::FASTA => write!(f, "FASTA"),
            Self::BED => write!(f, "BED"),
            Self::GFF => write!(f, "GFF"),
            Self::UNKNOWN => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ContainerKind {
    BGZF,
    GZIP,
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
    pub container: Option<ContainerKind>,
    pub confidence: Confidence,
    pub bam_magic_present: bool,
}

impl ProbeResult {
    fn new(
        detected_format: DetectedFormat,
        container: Option<ContainerKind>,
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

pub fn probe_path(path: &Path) -> Result<ProbeResult, BamanaError> {
    let mut file = File::open(path).map_err(|error| BamanaError::from_io(path, error))?;
    let mut prefix = vec![0_u8; 65_536];
    let bytes_read = file
        .read(&mut prefix)
        .map_err(|error| BamanaError::from_io(path, error))?;
    prefix.truncate(bytes_read);

    if prefix.is_empty() {
        return Ok(extension_hint(path)
            .unwrap_or_else(|| ProbeResult::new(DetectedFormat::UNKNOWN, None, Confidence::Low)));
    }

    if bgzf::is_bgzf_header(&prefix) {
        let bam_magic_present = bam::bam_magic_present(path).unwrap_or(false);
        if bam_magic_present {
            return Ok(ProbeResult::new(
                DetectedFormat::BAM,
                Some(ContainerKind::BGZF),
                Confidence::High,
            )
            .with_bam_magic(true));
        }

        return Ok(extension_hint(path).unwrap_or_else(|| {
            ProbeResult::new(
                DetectedFormat::UNKNOWN,
                Some(ContainerKind::BGZF),
                Confidence::Low,
            )
            .with_bam_magic(false)
        }));
    }

    if bgzf::is_gzip(&prefix) {
        if let Some(mut hint) = extension_hint(path) {
            if hint.detected_format == DetectedFormat::FastqGz {
                hint.container = Some(ContainerKind::GZIP);
                return Ok(hint);
            }
        }

        return Ok(ProbeResult::new(
            DetectedFormat::UNKNOWN,
            Some(ContainerKind::GZIP),
            Confidence::Low,
        ));
    }

    if prefix.starts_with(b"CRAM") {
        return Ok(ProbeResult::new(
            DetectedFormat::CRAM,
            None,
            Confidence::High,
        ));
    }

    let text = String::from_utf8_lossy(&prefix);
    if looks_like_fasta(&text) {
        return Ok(ProbeResult::new(
            DetectedFormat::FASTA,
            None,
            Confidence::High,
        ));
    }
    if looks_like_fastq(&text) {
        return Ok(ProbeResult::new(
            DetectedFormat::FASTQ,
            None,
            Confidence::High,
        ));
    }
    if looks_like_sam(&text) {
        return Ok(ProbeResult::new(
            DetectedFormat::SAM,
            None,
            Confidence::Medium,
        ));
    }
    if looks_like_gff(&text) {
        return Ok(ProbeResult::new(
            DetectedFormat::GFF,
            None,
            Confidence::Medium,
        ));
    }
    if looks_like_bed(&text) {
        return Ok(ProbeResult::new(
            DetectedFormat::BED,
            None,
            Confidence::Medium,
        ));
    }

    Ok(extension_hint(path)
        .unwrap_or_else(|| ProbeResult::new(DetectedFormat::UNKNOWN, None, Confidence::Low)))
}

fn extension_hint(path: &Path) -> Option<ProbeResult> {
    let path_string = path.to_string_lossy().to_ascii_lowercase();
    if path_string.ends_with(".bam") {
        return Some(ProbeResult::new(
            DetectedFormat::BAM,
            Some(ContainerKind::BGZF),
            Confidence::Medium,
        ));
    }
    if path_string.ends_with(".sam") {
        return Some(ProbeResult::new(
            DetectedFormat::SAM,
            None,
            Confidence::Medium,
        ));
    }
    if path_string.ends_with(".cram") {
        return Some(ProbeResult::new(
            DetectedFormat::CRAM,
            None,
            Confidence::Medium,
        ));
    }
    if path_string.ends_with(".fastq.gz") || path_string.ends_with(".fq.gz") {
        return Some(ProbeResult::new(
            DetectedFormat::FastqGz,
            Some(ContainerKind::GZIP),
            Confidence::Medium,
        ));
    }
    if path_string.ends_with(".fastq") || path_string.ends_with(".fq") {
        return Some(ProbeResult::new(
            DetectedFormat::FASTQ,
            None,
            Confidence::Medium,
        ));
    }
    if path_string.ends_with(".fasta")
        || path_string.ends_with(".fa")
        || path_string.ends_with(".fna")
    {
        return Some(ProbeResult::new(
            DetectedFormat::FASTA,
            None,
            Confidence::Medium,
        ));
    }
    if path_string.ends_with(".bed") {
        return Some(ProbeResult::new(
            DetectedFormat::BED,
            None,
            Confidence::Medium,
        ));
    }
    if path_string.ends_with(".gff") || path_string.ends_with(".gff3") {
        return Some(ProbeResult::new(
            DetectedFormat::GFF,
            None,
            Confidence::Medium,
        ));
    }
    None
}

fn looks_like_fasta(text: &str) -> bool {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.starts_with('>'))
}

fn looks_like_fastq(text: &str) -> bool {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    matches!(
        (lines.next(), lines.next(), lines.next()),
        (Some(first), Some(_second), Some(third)) if first.starts_with('@') && third.starts_with('+')
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
    use std::{fs, path::PathBuf};

    use crate::formats::{
        bgzf::test_support::build_bam_file,
        probe::{Confidence, DetectedFormat, probe_path},
    };

    fn write_test_file(name: &str, suffix: &str, bytes: &[u8]) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("bamana-{name}-{}{}", std::process::id(), suffix));
        fs::write(&path, bytes).expect("test fixture should be written");
        path
    }

    #[test]
    fn identifies_bam_from_bgzf_and_magic() {
        let bam = build_bam_file("@HD\tVN:1.6\n", &[("chr1", 10)]);
        let path = write_test_file("identify-bam", ".bam", &bam);
        let result = probe_path(&path).expect("probe should succeed");
        fs::remove_file(path).expect("test fixture should be removable");

        assert_eq!(result.detected_format, DetectedFormat::BAM);
        assert_eq!(result.confidence, Confidence::High);
        assert!(result.bam_magic_present);
    }

    #[test]
    fn identifies_fasta_from_text() {
        let path = write_test_file("identify-fasta", ".fa", b">chr1\nACGT\n");
        let result = probe_path(&path).expect("probe should succeed");
        fs::remove_file(path).expect("test fixture should be removable");

        assert_eq!(result.detected_format, DetectedFormat::FASTA);
    }
}
