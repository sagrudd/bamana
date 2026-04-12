use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::error::AppError;

const BAI_MAGIC: &[u8; 4] = b"BAI\x01";
const CSI_MAGIC: &[u8; 4] = b"CSI\x01";
const BAI_METADATA_BIN: u32 = 37_450;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IndexKind {
    #[serde(rename = "BAI")]
    Bai,
    #[serde(rename = "CSI")]
    Csi,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ResolvedIndex {
    pub path: PathBuf,
    pub kind: IndexKind,
}

#[derive(Debug, Clone)]
pub struct BaiReferenceSummary {
    pub mapped_reads: u64,
    pub unmapped_reads: u64,
}

#[derive(Debug, Clone)]
pub struct BaiIndexSummary {
    pub reference_summaries: Vec<Option<BaiReferenceSummary>>,
    pub unplaced_unmapped_reads: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CsiHeaderSummary {
    pub min_shift: i32,
    pub depth: i32,
    pub reference_count: i32,
}

#[derive(Debug, Clone)]
pub enum IndexResolution {
    Present(ResolvedIndex),
    Unsupported(ResolvedIndex),
    NotFound,
}

pub fn discover_index_candidates(path: &Path, prefer_csi: bool) -> Vec<ResolvedIndex> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for candidate_path in candidate_paths(path, prefer_csi) {
        if !candidate_path.is_file() {
            continue;
        }
        if !seen.insert(candidate_path.clone()) {
            continue;
        }

        let kind = detect_index_kind(&candidate_path).unwrap_or(IndexKind::Unknown);
        candidates.push(ResolvedIndex {
            path: candidate_path,
            kind,
        });
    }

    candidates
}

pub fn resolve_index_for_bam(path: &Path) -> IndexResolution {
    if let Some(candidate) = discover_index_candidates(path, false).into_iter().next() {
        match candidate.kind {
            IndexKind::Bai => return IndexResolution::Present(candidate),
            IndexKind::Csi | IndexKind::Unknown => return IndexResolution::Unsupported(candidate),
        }
    }

    IndexResolution::NotFound
}

pub fn default_index_output_path(bam_path: &Path, kind: IndexKind) -> Result<PathBuf, AppError> {
    match kind {
        IndexKind::Bai => Ok(PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()))),
        IndexKind::Csi => Ok(PathBuf::from(format!("{}.csi", bam_path.to_string_lossy()))),
        IndexKind::Unknown => Err(AppError::UnsupportedIndex {
            path: bam_path.to_path_buf(),
            detail: "Unknown index kind cannot be used to derive an output path.".to_string(),
        }),
    }
}

pub fn detect_index_kind(path: &Path) -> Result<IndexKind, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let mut reader = BufReader::new(file);
    let mut magic = [0_u8; 4];
    reader
        .read_exact(&mut magic)
        .map_err(|error| AppError::from_io(path, error))?;

    Ok(match &magic {
        BAI_MAGIC => IndexKind::Bai,
        CSI_MAGIC => IndexKind::Csi,
        _ => IndexKind::Unknown,
    })
}

pub fn parse_bai(path: &Path, expected_references: usize) -> Result<BaiIndexSummary, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let mut reader = BufReader::new(file);

    let mut magic = [0_u8; 4];
    reader
        .read_exact(&mut magic)
        .map_err(|error| AppError::from_io(path, error))?;

    if &magic == CSI_MAGIC {
        return Err(AppError::UnsupportedIndex {
            path: path.to_path_buf(),
            detail: "CSI indexes are detected but not implemented in this slice.".to_string(),
        });
    }
    if &magic != BAI_MAGIC {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "Index magic was not BAI\\1.".to_string(),
        });
    }

    let n_ref = read_i32(&mut reader, path)?;
    if n_ref < 0 {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "BAI reference count was negative.".to_string(),
        });
    }
    let n_ref = n_ref as usize;
    if n_ref != expected_references {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: format!(
                "BAI reference count {n_ref} does not match BAM header reference count {expected_references}."
            ),
        });
    }

    let mut reference_summaries = Vec::with_capacity(n_ref);
    for _ in 0..n_ref {
        let n_bin = read_i32(&mut reader, path)?;
        if n_bin < 0 {
            return Err(AppError::InvalidIndex {
                path: path.to_path_buf(),
                detail: "BAI bin count was negative.".to_string(),
            });
        }

        let mut summary = None;
        for _ in 0..(n_bin as usize) {
            let bin = read_u32(&mut reader, path)?;
            let n_chunk = read_i32(&mut reader, path)?;
            if n_chunk < 0 {
                return Err(AppError::InvalidIndex {
                    path: path.to_path_buf(),
                    detail: "BAI chunk count was negative.".to_string(),
                });
            }

            if bin == BAI_METADATA_BIN {
                if n_chunk != 2 {
                    return Err(AppError::InvalidIndex {
                        path: path.to_path_buf(),
                        detail: format!(
                            "BAI metadata pseudo-bin reported n_chunk={n_chunk}, expected 2."
                        ),
                    });
                }

                let _unmapped_beg = read_u64(&mut reader, path)?;
                let _unmapped_end = read_u64(&mut reader, path)?;
                let mapped_reads = read_u64(&mut reader, path)?;
                let unmapped_reads = read_u64(&mut reader, path)?;
                summary = Some(BaiReferenceSummary {
                    mapped_reads,
                    unmapped_reads,
                });
            } else {
                skip_bytes(&mut reader, path, (n_chunk as usize) * 16)?;
            }
        }

        let n_intv = read_i32(&mut reader, path)?;
        if n_intv < 0 {
            return Err(AppError::InvalidIndex {
                path: path.to_path_buf(),
                detail: "BAI interval count was negative.".to_string(),
            });
        }
        skip_bytes(&mut reader, path, (n_intv as usize) * 8)?;
        reference_summaries.push(summary);
    }

    let unplaced_unmapped_reads = read_optional_u64(&mut reader, path)?;

    Ok(BaiIndexSummary {
        reference_summaries,
        unplaced_unmapped_reads,
    })
}

pub fn parse_csi_header(path: &Path) -> Result<CsiHeaderSummary, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let mut reader = BufReader::new(file);

    let mut magic = [0_u8; 4];
    reader
        .read_exact(&mut magic)
        .map_err(|error| AppError::from_io(path, error))?;

    if &magic == BAI_MAGIC {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "Index magic was BAI\\1, not CSI\\1.".to_string(),
        });
    }
    if &magic != CSI_MAGIC {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "Index magic was not CSI\\1.".to_string(),
        });
    }

    let min_shift = read_i32(&mut reader, path)?;
    let depth = read_i32(&mut reader, path)?;
    let aux_length = read_i32(&mut reader, path)?;

    if min_shift < 0 {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "CSI min_shift was negative.".to_string(),
        });
    }
    if depth < 0 {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "CSI depth was negative.".to_string(),
        });
    }
    if aux_length < 0 {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "CSI aux length was negative.".to_string(),
        });
    }

    skip_bytes(&mut reader, path, aux_length as usize)?;

    let reference_count = read_i32(&mut reader, path)?;
    if reference_count < 0 {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "CSI reference count was negative.".to_string(),
        });
    }

    Ok(CsiHeaderSummary {
        min_shift,
        depth,
        reference_count,
    })
}

fn candidate_paths(path: &Path, prefer_csi: bool) -> Vec<PathBuf> {
    let bam_bai = PathBuf::from(format!("{}.bai", path.to_string_lossy()));
    let mut plain_bai = path.to_path_buf();
    plain_bai.set_extension("bai");

    let bam_csi = PathBuf::from(format!("{}.csi", path.to_string_lossy()));
    let mut plain_csi = path.to_path_buf();
    plain_csi.set_extension("csi");

    if prefer_csi {
        vec![bam_csi, plain_csi, bam_bai, plain_bai]
    } else {
        vec![bam_bai, plain_bai, bam_csi, plain_csi]
    }
}

fn read_i32(reader: &mut impl Read, path: &Path) -> Result<i32, AppError> {
    let mut bytes = [0_u8; 4];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| AppError::from_io(path, error))?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u32(reader: &mut impl Read, path: &Path) -> Result<u32, AppError> {
    let mut bytes = [0_u8; 4];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| AppError::from_io(path, error))?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read, path: &Path) -> Result<u64, AppError> {
    let mut bytes = [0_u8; 8];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| AppError::from_io(path, error))?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_optional_u64(reader: &mut impl Read, path: &Path) -> Result<Option<u64>, AppError> {
    let mut bytes = [0_u8; 8];
    match reader.read(&mut bytes[..1]) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(error) => return Err(AppError::from_io(path, error)),
    }
    reader
        .read_exact(&mut bytes[1..])
        .map_err(|error| AppError::from_io(path, error))?;
    Ok(Some(u64::from_le_bytes(bytes)))
}

fn skip_bytes(reader: &mut impl Read, path: &Path, mut len: usize) -> Result<(), AppError> {
    let mut buffer = [0_u8; 8192];
    while len > 0 {
        let chunk = len.min(buffer.len());
        reader
            .read_exact(&mut buffer[..chunk])
            .map_err(|error| AppError::from_io(path, error))?;
        len -= chunk;
    }
    Ok(())
}

#[cfg(test)]
pub mod test_support {
    use super::BAI_METADATA_BIN;

    pub fn build_bai_file(
        per_reference_counts: &[Option<(u64, u64)>],
        unplaced_unmapped_reads: Option<u64>,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BAI\x01");
        bytes.extend_from_slice(&(per_reference_counts.len() as i32).to_le_bytes());

        for entry in per_reference_counts {
            match entry {
                Some((mapped, unmapped)) => {
                    bytes.extend_from_slice(&1_i32.to_le_bytes());
                    bytes.extend_from_slice(&BAI_METADATA_BIN.to_le_bytes());
                    bytes.extend_from_slice(&2_i32.to_le_bytes());
                    bytes.extend_from_slice(&0_u64.to_le_bytes());
                    bytes.extend_from_slice(&0_u64.to_le_bytes());
                    bytes.extend_from_slice(&mapped.to_le_bytes());
                    bytes.extend_from_slice(&unmapped.to_le_bytes());
                }
                None => {
                    bytes.extend_from_slice(&0_i32.to_le_bytes());
                }
            }
            bytes.extend_from_slice(&0_i32.to_le_bytes());
        }

        if let Some(value) = unplaced_unmapped_reads {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        bytes
    }

    pub fn build_csi_header(reference_count: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"CSI\x01");
        bytes.extend_from_slice(&14_i32.to_le_bytes());
        bytes.extend_from_slice(&5_i32.to_le_bytes());
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.extend_from_slice(&reference_count.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::formats::bgzf::test_support::write_temp_file;

    use super::{
        IndexKind, IndexResolution, detect_index_kind, discover_index_candidates, parse_bai,
        parse_csi_header, resolve_index_for_bam, test_support,
    };

    #[test]
    fn parses_bai_pseudobin_counts() {
        let bai = test_support::build_bai_file(&[Some((12, 3)), Some((4, 1))], Some(5));
        let path = write_temp_file("map-index", "bai", &bai);
        let summary = parse_bai(&path, 2).expect("bai should parse");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(
            summary.reference_summaries[0]
                .as_ref()
                .map(|entry| entry.mapped_reads),
            Some(12)
        );
        assert_eq!(summary.unplaced_unmapped_reads, Some(5));
    }

    #[test]
    fn parses_csi_header() {
        let csi = test_support::build_csi_header(7);
        let path = write_temp_file("index-csi", "csi", &csi);
        let summary = parse_csi_header(&path).expect("csi header should parse");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(summary.reference_count, 7);
    }

    #[test]
    fn resolves_adjacent_bam_bai() {
        let bam = write_temp_file("map-index-path", "bam", b"");
        let bai_path = PathBuf::from(format!("{}.bai", bam.to_string_lossy()));
        fs::write(&bai_path, b"BAI\x01\x00\x00\x00\x00").expect("bai fixture should be written");

        let resolved = resolve_index_for_bam(&bam);

        fs::remove_file(bam).expect("bam fixture should be removable");
        fs::remove_file(bai_path).expect("bai fixture should be removable");

        assert!(matches!(resolved, IndexResolution::Present(_)));
    }

    #[test]
    fn detects_index_kind_from_magic() {
        let csi = test_support::build_csi_header(1);
        let path = write_temp_file("index-kind", "csi", &csi);
        let kind = detect_index_kind(&path).expect("kind should be detected");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(kind, IndexKind::Csi);
    }

    #[test]
    fn discovers_candidates_with_preferred_order() {
        let bam = write_temp_file("discover-index", "bam", b"");
        let bai_path = PathBuf::from(format!("{}.bai", bam.to_string_lossy()));
        let csi_path = PathBuf::from(format!("{}.csi", bam.to_string_lossy()));
        fs::write(&bai_path, b"BAI\x01\x00\x00\x00\x00").expect("bai fixture should be written");
        fs::write(&csi_path, test_support::build_csi_header(0))
            .expect("csi fixture should be written");

        let preferred = discover_index_candidates(&bam, true);

        fs::remove_file(&bam).expect("bam fixture should be removable");
        fs::remove_file(bai_path).expect("bai fixture should be removable");
        fs::remove_file(csi_path).expect("csi fixture should be removable");

        assert!(matches!(
            preferred.first().map(|entry| entry.kind),
            Some(IndexKind::Csi)
        ));
    }
}
