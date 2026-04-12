use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use flate2::read::GzDecoder;

use crate::error::BamanaError;

pub const BGZF_EOF_MARKER: [u8; 28] = [
    31, 139, 8, 4, 0, 0, 0, 0, 0, 255, 6, 0, 66, 67, 2, 0, 27, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn is_gzip(bytes: &[u8]) -> bool {
    bytes.len() >= 3 && bytes[0] == 0x1f && bytes[1] == 0x8b && bytes[2] == 0x08
}

pub fn is_bgzf_header(bytes: &[u8]) -> bool {
    bgzf_block_size(bytes).is_some()
}

pub fn bgzf_block_size(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 18 || !is_gzip(bytes) || bytes[3] & 0x04 == 0 {
        return None;
    }

    let xlen = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
    if bytes.len() < 12 + xlen {
        return None;
    }

    let mut cursor = 12;
    let end = 12 + xlen;
    while cursor + 4 <= end {
        let subfield_len = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]) as usize;
        let payload_start = cursor + 4;
        let payload_end = payload_start + subfield_len;
        if payload_end > end {
            return None;
        }

        if bytes[cursor] == b'B' && bytes[cursor + 1] == b'C' && subfield_len == 2 {
            let bsize =
                u16::from_le_bytes([bytes[payload_start], bytes[payload_start + 1]]) as usize;
            return Some(bsize + 1);
        }

        cursor = payload_end;
    }

    None
}

pub fn has_canonical_eof_marker(path: &Path) -> Result<bool, BamanaError> {
    let mut file = File::open(path).map_err(|error| BamanaError::from_io(path, error))?;
    let file_size = file
        .metadata()
        .map_err(|error| BamanaError::from_io(path, error))?
        .len();

    if file_size < BGZF_EOF_MARKER.len() as u64 {
        return Err(BamanaError::TruncatedFile {
            path: path.to_path_buf(),
            detail: "File is smaller than the canonical 28-byte BGZF EOF marker.".to_string(),
        });
    }

    file.seek(SeekFrom::End(-(BGZF_EOF_MARKER.len() as i64)))
        .map_err(|error| BamanaError::from_io(path, error))?;

    let mut tail = [0_u8; BGZF_EOF_MARKER.len()];
    file.read_exact(&mut tail)
        .map_err(|error| BamanaError::from_io(path, error))?;

    Ok(tail == BGZF_EOF_MARKER)
}

pub fn read_first_bgzf_payload(path: &Path) -> Result<Vec<u8>, BamanaError> {
    let mut file = File::open(path).map_err(|error| BamanaError::from_io(path, error))?;
    let member = read_bgzf_member(&mut file, path)?;
    let member = member.ok_or_else(|| BamanaError::TruncatedFile {
        path: path.to_path_buf(),
        detail: "File did not contain a BGZF member.".to_string(),
    })?;
    decompress_member(&member, path)
}

pub fn read_bgzf_member(file: &mut File, path: &Path) -> Result<Option<Vec<u8>>, BamanaError> {
    let mut fixed_header = [0_u8; 12];
    match file.read(&mut fixed_header[..1]) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(error) => return Err(BamanaError::from_io(path, error)),
    }

    file.read_exact(&mut fixed_header[1..])
        .map_err(|error| BamanaError::from_io(path, error))?;

    let xlen = u16::from_le_bytes([fixed_header[10], fixed_header[11]]) as usize;
    let mut extra = vec![0_u8; xlen];
    file.read_exact(&mut extra)
        .map_err(|error| BamanaError::from_io(path, error))?;

    let mut header = Vec::with_capacity(12 + extra.len());
    header.extend_from_slice(&fixed_header);
    header.extend_from_slice(&extra);

    let block_size = bgzf_block_size(&header).ok_or_else(|| BamanaError::InvalidBam {
        path: path.to_path_buf(),
        detail: "First member does not contain a valid BGZF block header.".to_string(),
    })?;

    if block_size < header.len() {
        return Err(BamanaError::InvalidBam {
            path: path.to_path_buf(),
            detail: "BGZF block size is smaller than the header length.".to_string(),
        });
    }

    let mut member = vec![0_u8; block_size];
    member[..header.len()].copy_from_slice(&header);
    file.read_exact(&mut member[header.len()..])
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                BamanaError::TruncatedFile {
                    path: path.to_path_buf(),
                    detail: "BGZF block ended before the declared block size.".to_string(),
                }
            } else {
                BamanaError::from_io(path, error)
            }
        })?;

    Ok(Some(member))
}

pub fn decompress_member(member: &[u8], path: &Path) -> Result<Vec<u8>, BamanaError> {
    let mut decoder = GzDecoder::new(member);
    let mut payload = Vec::new();
    decoder
        .read_to_end(&mut payload)
        .map_err(|error| BamanaError::InvalidBam {
            path: path.to_path_buf(),
            detail: format!("Unable to decompress BGZF member: {error}"),
        })?;
    Ok(payload)
}

pub fn read_decompressed_prefix(
    path: &Path,
    minimum_bytes: usize,
    limit_bytes: usize,
) -> Result<Vec<u8>, BamanaError> {
    let mut file = File::open(path).map_err(|error| BamanaError::from_io(path, error))?;
    let mut buffer = Vec::new();

    while buffer.len() < minimum_bytes && buffer.len() < limit_bytes {
        let Some(member) = read_bgzf_member(&mut file, path)? else {
            break;
        };
        let payload = decompress_member(&member, path)?;
        buffer.extend_from_slice(&payload);
        if payload.is_empty() {
            break;
        }
    }

    if buffer.len() > limit_bytes {
        buffer.truncate(limit_bytes);
    }

    Ok(buffer)
}

#[cfg(test)]
pub mod test_support {
    use std::io::Write;

    use flate2::{Compression, GzBuilder};

    use super::BGZF_EOF_MARKER;

    pub fn build_bgzf_member(payload: &[u8]) -> Vec<u8> {
        let extra = [b'B', b'C', 2, 0, 0, 0];
        let mut encoder = GzBuilder::new()
            .extra(extra.as_slice())
            .write(Vec::new(), Compression::default());
        encoder
            .write_all(payload)
            .expect("BGZF test payload should compress");
        let mut member = encoder.finish().expect("BGZF encoder should finish");
        let bsize = (member.len() - 1) as u16;
        member[16..18].copy_from_slice(&bsize.to_le_bytes());
        member
    }

    pub fn build_bam_file(header_text: &str, references: &[(&str, u32)]) -> Vec<u8> {
        let mut bam = Vec::new();
        bam.extend_from_slice(b"BAM\x01");
        bam.extend_from_slice(&(header_text.len() as i32).to_le_bytes());
        bam.extend_from_slice(header_text.as_bytes());
        bam.extend_from_slice(&(references.len() as i32).to_le_bytes());

        for (name, length) in references {
            let mut nul_terminated_name = name.as_bytes().to_vec();
            nul_terminated_name.push(0);
            bam.extend_from_slice(&(nul_terminated_name.len() as i32).to_le_bytes());
            bam.extend_from_slice(&nul_terminated_name);
            bam.extend_from_slice(&(*length as i32).to_le_bytes());
        }

        let mut bytes = build_bgzf_member(&bam);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        bytes
    }
}
