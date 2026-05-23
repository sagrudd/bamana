use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    bam::scan::{BamRecordVirtualOffsets, BamScanner},
    bgzf::virtual_offset::VirtualOffset,
    error::AppError,
};

const BAI_MAGIC: &[u8; 4] = b"BAI\x01";
const CSI_MAGIC: &[u8; 4] = b"CSI\x01";
const BAI_METADATA_BIN: u32 = 37_450;
const BAI_MAX_REGULAR_BIN: u32 = BAI_METADATA_BIN - 1;
const BAI_LINEAR_WINDOW_SHIFT: u32 = 14;
const BAI_MAX_POSITION: u32 = 1 << 29;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IndexKind {
    #[serde(rename = "BAI")]
    Bai,
    #[serde(rename = "CSI")]
    Csi,
    #[serde(rename = "GZI")]
    Gzi,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaiChunk {
    pub start: VirtualOffset,
    pub end: VirtualOffset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaiReferenceIndex {
    pub bins: BTreeMap<u32, Vec<BaiChunk>>,
    pub linear_index: Vec<VirtualOffset>,
    pub mapped_reads: u64,
    pub unmapped_reads: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaiIndex {
    pub references: Vec<BaiReferenceIndex>,
    pub unplaced_unmapped_reads: u64,
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
            IndexKind::Csi | IndexKind::Gzi | IndexKind::Unknown => {
                return IndexResolution::Unsupported(candidate);
            }
        }
    }

    IndexResolution::NotFound
}

pub fn default_index_output_path(bam_path: &Path, kind: IndexKind) -> Result<PathBuf, AppError> {
    match kind {
        IndexKind::Bai => Ok(PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()))),
        IndexKind::Csi => Ok(PathBuf::from(format!("{}.csi", bam_path.to_string_lossy()))),
        IndexKind::Gzi => {
            let mut path = bam_path.to_path_buf();
            path.set_extension("gzi");
            Ok(path)
        }
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

pub fn bam_newer_than_index(bam_path: &Path, index_path: &Path) -> Option<bool> {
    let bam_modified = fs::metadata(bam_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())?;
    let index_modified = fs::metadata(index_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())?;
    Some(bam_modified > index_modified)
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
        let mut seen_bins = HashSet::new();
        for _ in 0..(n_bin as usize) {
            let bin = read_u32(&mut reader, path)?;
            let n_chunk = read_i32(&mut reader, path)?;
            if n_chunk < 0 {
                return Err(AppError::InvalidIndex {
                    path: path.to_path_buf(),
                    detail: "BAI chunk count was negative.".to_string(),
                });
            }
            if !seen_bins.insert(bin) {
                return Err(AppError::InvalidIndex {
                    path: path.to_path_buf(),
                    detail: format!("BAI reference contained duplicate bin {bin}."),
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
                validate_regular_bai_bin(&mut reader, path, bin, n_chunk as usize)?;
            }
        }

        let n_intv = read_i32(&mut reader, path)?;
        if n_intv < 0 {
            return Err(AppError::InvalidIndex {
                path: path.to_path_buf(),
                detail: "BAI interval count was negative.".to_string(),
            });
        }
        validate_bai_linear_index(&mut reader, path, n_intv as usize)?;
        reference_summaries.push(summary);
    }

    let unplaced_unmapped_reads = read_optional_u64(&mut reader, path)?;
    ensure_no_trailing_bytes(&mut reader, path)?;

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

pub fn build_bai_index_from_bam(path: &Path) -> Result<BaiIndex, AppError> {
    let mut scanner = BamScanner::open(path)?;
    let reference_count = scanner.header().header.references.len();
    let mut builder = BaiIndexBuilder::new(path, reference_count);

    while let Some(positioned) = scanner.next_record_with_virtual_offsets()? {
        builder.add_record(
            positioned.record.ref_id(),
            positioned.record.pos(),
            positioned.record.flag_summary().is_unmapped,
            positioned.record.cigar_bytes(),
            positioned.virtual_offsets,
        )?;
    }

    Ok(builder.finish())
}

pub fn write_bai_index(path: &Path, index: &BaiIndex) -> Result<(), AppError> {
    let file = File::create(path).map_err(|error| AppError::WriteError {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut writer = BufWriter::new(file);
    write_bai_index_to_writer(path, index, &mut writer)?;
    writer.flush().map_err(|error| AppError::WriteError {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn write_bai_index_to_writer(
    path: &Path,
    index: &BaiIndex,
    writer: &mut impl Write,
) -> Result<(), AppError> {
    write_all(path, writer, BAI_MAGIC)?;
    write_i32_count(path, writer, index.references.len(), "BAI reference count")?;

    for reference in &index.references {
        let bin_count =
            reference
                .bins
                .len()
                .checked_add(1)
                .ok_or_else(|| AppError::InvalidIndex {
                    path: path.to_path_buf(),
                    detail: "BAI bin count overflowed while adding metadata pseudo-bin."
                        .to_string(),
                })?;
        write_i32_count(path, writer, bin_count, "BAI bin count")?;

        for (bin, chunks) in &reference.bins {
            write_all(path, writer, &bin.to_le_bytes())?;
            write_i32_count(path, writer, chunks.len(), "BAI chunk count")?;
            for chunk in chunks {
                write_all(path, writer, &chunk.start.packed().to_le_bytes())?;
                write_all(path, writer, &chunk.end.packed().to_le_bytes())?;
            }
        }

        let (reference_start, reference_end) = reference_virtual_span(reference);
        write_all(path, writer, &BAI_METADATA_BIN.to_le_bytes())?;
        write_all(path, writer, &2_i32.to_le_bytes())?;
        write_all(path, writer, &reference_start.to_le_bytes())?;
        write_all(path, writer, &reference_end.to_le_bytes())?;
        write_all(path, writer, &reference.mapped_reads.to_le_bytes())?;
        write_all(path, writer, &reference.unmapped_reads.to_le_bytes())?;

        write_i32_count(
            path,
            writer,
            reference.linear_index.len(),
            "BAI linear interval count",
        )?;
        for offset in &reference.linear_index {
            write_all(path, writer, &offset.packed().to_le_bytes())?;
        }
    }

    write_all(path, writer, &index.unplaced_unmapped_reads.to_le_bytes())
}

fn reference_virtual_span(reference: &BaiReferenceIndex) -> (u64, u64) {
    let mut start = None;
    let mut end = VirtualOffset::ZERO;

    for chunks in reference.bins.values() {
        for chunk in chunks {
            if match start {
                Some(current) => chunk.start < current,
                None => true,
            } {
                start = Some(chunk.start);
            }
            if chunk.end > end {
                end = chunk.end;
            }
        }
    }

    (start.unwrap_or(VirtualOffset::ZERO).packed(), end.packed())
}

fn write_i32_count(
    path: &Path,
    writer: &mut impl Write,
    count: usize,
    label: &str,
) -> Result<(), AppError> {
    let count = i32::try_from(count).map_err(|_| AppError::InvalidIndex {
        path: path.to_path_buf(),
        detail: format!("{label} exceeded the BAI i32 limit."),
    })?;
    write_all(path, writer, &count.to_le_bytes())
}

fn write_all(path: &Path, writer: &mut impl Write, bytes: &[u8]) -> Result<(), AppError> {
    writer
        .write_all(bytes)
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

pub fn bai_bin_for_region(start: u32, end: u32) -> Result<u32, AppError> {
    let end = normalize_region_end(start, end, Path::new("<region>"))?;
    Ok(bai_bin_for_normalized_region(start, end))
}

pub fn bai_bins_for_region(start: u32, end: u32) -> Result<Vec<u32>, AppError> {
    let end = normalize_region_end(start, end, Path::new("<region>"))?;
    Ok(bai_bins_for_normalized_region(start, end))
}

#[derive(Debug)]
struct BaiIndexBuilder {
    path: PathBuf,
    references: Vec<BaiReferenceIndex>,
    unplaced_unmapped_reads: u64,
    last_mapped_coordinate: Option<(i32, i32)>,
}

impl BaiIndexBuilder {
    fn new(path: &Path, reference_count: usize) -> Self {
        Self {
            path: path.to_path_buf(),
            references: (0..reference_count)
                .map(|_| BaiReferenceIndex {
                    bins: BTreeMap::new(),
                    linear_index: Vec::new(),
                    mapped_reads: 0,
                    unmapped_reads: 0,
                })
                .collect(),
            unplaced_unmapped_reads: 0,
            last_mapped_coordinate: None,
        }
    }

    fn add_record(
        &mut self,
        ref_id: i32,
        pos: i32,
        is_unmapped: bool,
        cigar_bytes: &[u8],
        offsets: BamRecordVirtualOffsets,
    ) -> Result<(), AppError> {
        if is_unmapped {
            if ref_id >= 0 {
                let reference = self.reference_mut(ref_id)?;
                reference.unmapped_reads += 1;
            } else {
                self.unplaced_unmapped_reads += 1;
            }
            return Ok(());
        }

        if ref_id < 0 {
            return Err(AppError::InvalidIndex {
                path: self.path.clone(),
                detail: "Mapped BAM record had a negative reference id.".to_string(),
            });
        }
        if pos < 0 {
            return Err(AppError::InvalidIndex {
                path: self.path.clone(),
                detail: "Mapped BAM record had a negative coordinate.".to_string(),
            });
        }

        if let Some((last_ref, last_pos)) = self.last_mapped_coordinate {
            if ref_id < last_ref || (ref_id == last_ref && pos < last_pos) {
                return Err(AppError::InvalidIndex {
                    path: self.path.clone(),
                    detail:
                        "BAM records are not in coordinate order; BAI construction requires sorted mapped records."
                            .to_string(),
                });
            }
        }
        self.last_mapped_coordinate = Some((ref_id, pos));

        let start = pos as u32;
        let span = reference_span(cigar_bytes, &self.path)?;
        let end = start
            .checked_add(span)
            .ok_or_else(|| AppError::InvalidIndex {
                path: self.path.clone(),
                detail: "Mapped BAM record reference span overflowed u32.".to_string(),
            })?;
        let end = normalize_region_end(start, end, &self.path)?;
        let bin = bai_bin_for_normalized_region(start, end);

        let reference = self.reference_mut(ref_id)?;
        reference.mapped_reads += 1;
        push_chunk(
            &mut reference.bins,
            bin,
            BaiChunk {
                start: offsets.start,
                end: offsets.end,
            },
        );
        update_linear_index(&mut reference.linear_index, start, end, offsets.start);

        Ok(())
    }

    fn reference_mut(&mut self, ref_id: i32) -> Result<&mut BaiReferenceIndex, AppError> {
        let index = usize::try_from(ref_id).map_err(|_| AppError::InvalidIndex {
            path: self.path.clone(),
            detail: "BAM record reference id could not be represented as an index.".to_string(),
        })?;
        self.references
            .get_mut(index)
            .ok_or_else(|| AppError::InvalidIndex {
                path: self.path.clone(),
                detail: format!(
                    "BAM record reference id {ref_id} is outside the header reference dictionary."
                ),
            })
    }

    fn finish(self) -> BaiIndex {
        BaiIndex {
            references: self.references,
            unplaced_unmapped_reads: self.unplaced_unmapped_reads,
        }
    }
}

fn reference_span(cigar_bytes: &[u8], path: &Path) -> Result<u32, AppError> {
    if cigar_bytes.is_empty() {
        return Ok(1);
    }

    let mut span = 0_u32;
    for chunk in cigar_bytes.chunks_exact(4) {
        let raw = u32::from_le_bytes(chunk.try_into().expect("chunk size checked"));
        let op_len = raw >> 4;
        let op = raw & 0x0f;
        if matches!(op, 0 | 2 | 3 | 7 | 8) {
            span = span
                .checked_add(op_len)
                .ok_or_else(|| AppError::InvalidIndex {
                    path: path.to_path_buf(),
                    detail: "BAM CIGAR reference span overflowed u32.".to_string(),
                })?;
        }
    }

    Ok(span.max(1))
}

fn normalize_region_end(start: u32, end: u32, path: &Path) -> Result<u32, AppError> {
    let end = end.max(start.saturating_add(1));
    if end > BAI_MAX_POSITION {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: format!(
                "BAI supports coordinates below {BAI_MAX_POSITION}; record ended at {end}."
            ),
        });
    }
    Ok(end)
}

fn bai_bin_for_normalized_region(start: u32, end: u32) -> u32 {
    let end = end - 1;
    if start >> 14 == end >> 14 {
        return 4_681 + (start >> 14);
    }
    if start >> 17 == end >> 17 {
        return 585 + (start >> 17);
    }
    if start >> 20 == end >> 20 {
        return 73 + (start >> 20);
    }
    if start >> 23 == end >> 23 {
        return 9 + (start >> 23);
    }
    if start >> 26 == end >> 26 {
        return 1 + (start >> 26);
    }
    0
}

fn bai_bins_for_normalized_region(start: u32, end: u32) -> Vec<u32> {
    let end = end - 1;
    let mut bins = Vec::new();
    bins.push(0);
    for bin in (1 + (start >> 26))..=(1 + (end >> 26)) {
        bins.push(bin);
    }
    for bin in (9 + (start >> 23))..=(9 + (end >> 23)) {
        bins.push(bin);
    }
    for bin in (73 + (start >> 20))..=(73 + (end >> 20)) {
        bins.push(bin);
    }
    for bin in (585 + (start >> 17))..=(585 + (end >> 17)) {
        bins.push(bin);
    }
    for bin in (4_681 + (start >> 14))..=(4_681 + (end >> 14)) {
        bins.push(bin);
    }
    bins
}

fn push_chunk(bins: &mut BTreeMap<u32, Vec<BaiChunk>>, bin: u32, chunk: BaiChunk) {
    let chunks = bins.entry(bin).or_default();
    if let Some(last) = chunks.last_mut() {
        if chunk.start <= last.end {
            if chunk.end > last.end {
                last.end = chunk.end;
            }
            return;
        }
    }
    chunks.push(chunk);
}

fn update_linear_index(
    linear_index: &mut Vec<VirtualOffset>,
    start: u32,
    end: u32,
    record_start: VirtualOffset,
) {
    let first_window = (start >> BAI_LINEAR_WINDOW_SHIFT) as usize;
    let last_window = ((end - 1) >> BAI_LINEAR_WINDOW_SHIFT) as usize;
    if linear_index.len() <= last_window {
        linear_index.resize(last_window + 1, VirtualOffset::ZERO);
    }

    for entry in &mut linear_index[first_window..=last_window] {
        if *entry == VirtualOffset::ZERO || record_start < *entry {
            *entry = record_start;
        }
    }
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

fn validate_regular_bai_bin(
    reader: &mut impl Read,
    path: &Path,
    bin: u32,
    chunk_count: usize,
) -> Result<(), AppError> {
    if bin > BAI_MAX_REGULAR_BIN {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: format!("BAI regular bin {bin} exceeds maximum bin {BAI_MAX_REGULAR_BIN}."),
        });
    }
    if chunk_count == 0 {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: format!("BAI regular bin {bin} reported zero chunks."),
        });
    }

    let mut previous_end = None;
    for chunk_index in 0..chunk_count {
        let start = read_u64(reader, path)?;
        let end = read_u64(reader, path)?;
        if start >= end {
            return Err(AppError::InvalidIndex {
                path: path.to_path_buf(),
                detail: format!(
                    "BAI chunk {chunk_index} in bin {bin} had start virtual offset {start} that was not before end offset {end}."
                ),
            });
        }
        if let Some(previous_end) = previous_end {
            if start < previous_end {
                return Err(AppError::InvalidIndex {
                    path: path.to_path_buf(),
                    detail: format!("BAI chunks in bin {bin} were not ordered by virtual offset."),
                });
            }
        }
        previous_end = Some(end);
    }

    Ok(())
}

fn validate_bai_linear_index(
    reader: &mut impl Read,
    path: &Path,
    interval_count: usize,
) -> Result<(), AppError> {
    let mut previous_nonzero = 0_u64;
    for interval_index in 0..interval_count {
        let offset = read_u64(reader, path)?;
        if offset == 0 {
            continue;
        }
        if previous_nonzero != 0 && offset < previous_nonzero {
            return Err(AppError::InvalidIndex {
                path: path.to_path_buf(),
                detail: format!(
                    "BAI linear-index interval {interval_index} moved backward in virtual-offset order."
                ),
            });
        }
        previous_nonzero = offset;
    }

    Ok(())
}

fn ensure_no_trailing_bytes(reader: &mut impl Read, path: &Path) -> Result<(), AppError> {
    let mut byte = [0_u8; 1];
    match reader.read(&mut byte) {
        Ok(0) => Ok(()),
        Ok(_) => Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "BAI contained trailing bytes after the optional unplaced-unmapped count."
                .to_string(),
        }),
        Err(error) => Err(AppError::from_io(path, error)),
    }
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

    use crate::{
        error::AppError,
        formats::bgzf::test_support::{
            build_bam_file_with_header_and_records, build_light_record, write_temp_file,
        },
    };

    use super::{
        IndexKind, IndexResolution, bai_bin_for_region, build_bai_index_from_bam,
        detect_index_kind, discover_index_candidates, parse_bai, parse_csi_header,
        resolve_index_for_bam, test_support, write_bai_index,
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
    fn computes_representative_bai_bins() {
        assert_eq!(bai_bin_for_region(0, 1).expect("bin should compute"), 4_681);
        assert_eq!(
            bai_bin_for_region(16_384, 16_385).expect("bin should compute"),
            4_682
        );
        assert_eq!(
            bai_bin_for_region(0, 20_000).expect("bin should compute"),
            585
        );
        assert_eq!(
            bai_bin_for_region(0, 140_000).expect("bin should compute"),
            73
        );
    }

    #[test]
    fn builds_bai_chunks_and_linear_index_from_scanner_offsets() {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(0, 8, "read2", 0);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[first, second],
        );
        let path = write_temp_file("bai-build-basic", "bam", &bytes);

        let index = build_bai_index_from_bam(&path).expect("index should build");
        fs::remove_file(path).expect("fixture should be removable");

        let reference = &index.references[0];
        assert_eq!(reference.mapped_reads, 2);
        assert_eq!(reference.unmapped_reads, 0);
        assert_eq!(reference.bins.len(), 1);
        let chunks = reference.bins.get(&4_681).expect("bin should exist");
        assert_eq!(chunks.len(), 1, "adjacent chunks should merge");
        assert!(chunks[0].start < chunks[0].end);
        assert_eq!(reference.linear_index.len(), 1);
        assert_eq!(reference.linear_index[0], chunks[0].start);
    }

    #[test]
    fn builds_linear_index_across_windows() {
        let first = build_light_record(0, 5, "read1", 0);
        let second = build_light_record(0, 20_000, "read2", 0);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[first, second],
        );
        let path = write_temp_file("bai-build-linear", "bam", &bytes);

        let index = build_bai_index_from_bam(&path).expect("index should build");
        fs::remove_file(path).expect("fixture should be removable");

        let reference = &index.references[0];
        assert_eq!(reference.mapped_reads, 2);
        assert!(reference.bins.contains_key(&4_681));
        assert!(reference.bins.contains_key(&4_682));
        assert_eq!(reference.linear_index.len(), 2);
        assert!(reference.linear_index[0] < reference.linear_index[1]);
    }

    #[test]
    fn accounts_for_reference_unmapped_and_unplaced_reads() {
        let mapped = build_light_record(0, 5, "mapped", 0);
        let reference_unmapped = build_light_record(0, 0, "ref_unmapped", 0x4);
        let unplaced = build_light_record(-1, -1, "unplaced", 0x4);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[mapped, reference_unmapped, unplaced],
        );
        let path = write_temp_file("bai-build-unmapped", "bam", &bytes);

        let index = build_bai_index_from_bam(&path).expect("index should build");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(index.references[0].mapped_reads, 1);
        assert_eq!(index.references[0].unmapped_reads, 1);
        assert_eq!(index.unplaced_unmapped_reads, 1);
    }

    #[test]
    fn writes_bai_with_metadata_counts_and_trailing_unplaced_count() {
        let mapped = build_light_record(0, 5, "mapped", 0);
        let reference_unmapped = build_light_record(0, 0, "ref_unmapped", 0x4);
        let unplaced = build_light_record(-1, -1, "unplaced", 0x4);
        let bytes = build_bam_file_with_header_and_records(
            "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[mapped, reference_unmapped, unplaced],
        );
        let bam_path = write_temp_file("bai-write-source", "bam", &bytes);
        let bai_path = PathBuf::from(format!("{}.bai", bam_path.to_string_lossy()));

        let index = build_bai_index_from_bam(&bam_path).expect("index should build");
        write_bai_index(&bai_path, &index).expect("index should write");
        let summary = parse_bai(&bai_path, 1).expect("written index should parse");

        fs::remove_file(bam_path).expect("fixture should be removable");
        fs::remove_file(bai_path).expect("written index should be removable");

        assert_eq!(
            summary.reference_summaries[0]
                .as_ref()
                .map(|entry| (entry.mapped_reads, entry.unmapped_reads)),
            Some((1, 1))
        );
        assert_eq!(summary.unplaced_unmapped_reads, Some(1));
    }

    #[test]
    fn rejects_bai_chunk_with_backward_virtual_offsets() {
        let bai = build_bai_with_regular_bin(4_681, &[(20, 10)], &[], Some(0));
        let path = write_temp_file("bai-bad-chunk-order", "bai", &bai);

        let error = parse_bai(&path, 1).expect_err("bad chunk should fail");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(
            error
                .to_json_error()
                .detail
                .is_some_and(|detail| detail.contains("start virtual offset"))
        );
    }

    #[test]
    fn rejects_bai_chunks_not_sorted_within_bin() {
        let bai = build_bai_with_regular_bin(4_681, &[(10, 30), (20, 40)], &[], Some(0));
        let path = write_temp_file("bai-overlap-chunks", "bai", &bai);

        let error = parse_bai(&path, 1).expect_err("overlapping chunk order should fail");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(
            error
                .to_json_error()
                .detail
                .is_some_and(|detail| detail.contains("not ordered"))
        );
    }

    #[test]
    fn rejects_bai_linear_index_that_moves_backward() {
        let bai = build_bai_with_regular_bin(4_681, &[(10, 30)], &[50, 40], Some(0));
        let path = write_temp_file("bai-bad-linear", "bai", &bai);

        let error = parse_bai(&path, 1).expect_err("linear index should fail");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(
            error
                .to_json_error()
                .detail
                .is_some_and(|detail| detail.contains("linear-index"))
        );
    }

    #[test]
    fn rejects_unsorted_mapped_records() {
        let first = build_light_record(0, 20, "read1", 0);
        let second = build_light_record(0, 10, "read2", 0);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[first, second],
        );
        let path = write_temp_file("bai-build-unsorted", "bam", &bytes);

        let error = build_bai_index_from_bam(&path).expect_err("unsorted BAM should fail");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(matches!(error, AppError::InvalidIndex { .. }));
        assert!(
            error
                .to_json_error()
                .detail
                .is_some_and(|detail| detail.contains("coordinate order"))
        );
    }

    #[test]
    fn rejects_mapped_record_outside_reference_dictionary() {
        let record = build_light_record(1, 5, "read1", 0);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:100000\n",
            &[("chr1", 100000)],
            &[record],
        );
        let path = write_temp_file("bai-build-bad-ref", "bam", &bytes);

        let error = build_bai_index_from_bam(&path).expect_err("bad reference id should fail");
        fs::remove_file(path).expect("fixture should be removable");

        assert!(
            error
                .to_json_error()
                .detail
                .is_some_and(|detail| detail.contains("outside the header reference dictionary"))
        );
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

    fn build_bai_with_regular_bin(
        bin: u32,
        chunks: &[(u64, u64)],
        linear_index: &[u64],
        unplaced_unmapped_reads: Option<u64>,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BAI\x01");
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        bytes.extend_from_slice(&bin.to_le_bytes());
        bytes.extend_from_slice(&(chunks.len() as i32).to_le_bytes());
        for (start, end) in chunks {
            bytes.extend_from_slice(&start.to_le_bytes());
            bytes.extend_from_slice(&end.to_le_bytes());
        }
        bytes.extend_from_slice(&(linear_index.len() as i32).to_le_bytes());
        for offset in linear_index {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        if let Some(value) = unplaced_unmapped_reads {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}
