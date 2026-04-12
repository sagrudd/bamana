use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use flate2::read::GzDecoder;

use crate::error::AppError;

pub const BGZF_EOF_MARKER: [u8; 28] = [
    31, 139, 8, 4, 0, 0, 0, 0, 0, 255, 6, 0, 66, 67, 2, 0, 27, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn is_gzip_signature(bytes: &[u8]) -> bool {
    bytes.len() >= 3 && bytes[0] == 0x1f && bytes[1] == 0x8b && bytes[2] == 0x08
}

pub fn is_bgzf_header(bytes: &[u8]) -> bool {
    bgzf_block_size(bytes).is_some()
}

pub fn has_bgzf_eof(path: &Path) -> Result<bool, AppError> {
    let mut file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let file_len = file
        .metadata()
        .map_err(|error| AppError::from_io(path, error))?
        .len();

    if file_len < BGZF_EOF_MARKER.len() as u64 {
        return Err(AppError::TruncatedFile {
            path: path.to_path_buf(),
            detail: "File is smaller than the canonical 28-byte BGZF EOF marker.".to_string(),
        });
    }

    file.seek(SeekFrom::End(-(BGZF_EOF_MARKER.len() as i64)))
        .map_err(|error| AppError::from_io(path, error))?;

    let mut tail = [0_u8; BGZF_EOF_MARKER.len()];
    file.read_exact(&mut tail)
        .map_err(|error| AppError::from_io(path, error))?;

    Ok(tail == BGZF_EOF_MARKER)
}

pub fn first_member_starts_with_bam_magic(path: &Path) -> Result<bool, AppError> {
    let payload = read_first_bgzf_payload(path)?;
    Ok(payload.starts_with(b"BAM\x01"))
}

fn read_first_bgzf_payload(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let member = read_bgzf_member(&mut file, path)?.ok_or_else(|| AppError::TruncatedFile {
        path: path.to_path_buf(),
        detail: "File did not contain a readable BGZF member.".to_string(),
    })?;

    decompress_member(&member, path)
}

fn read_bgzf_member(file: &mut File, path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    let mut fixed_header = [0_u8; 12];
    match file.read(&mut fixed_header[..1]) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(error) => return Err(AppError::from_io(path, error)),
    }

    file.read_exact(&mut fixed_header[1..])
        .map_err(|error| AppError::from_io(path, error))?;

    let xlen = u16::from_le_bytes([fixed_header[10], fixed_header[11]]) as usize;
    let mut extra = vec![0_u8; xlen];
    file.read_exact(&mut extra)
        .map_err(|error| AppError::from_io(path, error))?;

    let mut header = Vec::with_capacity(12 + extra.len());
    header.extend_from_slice(&fixed_header);
    header.extend_from_slice(&extra);

    let block_size = bgzf_block_size(&header).ok_or_else(|| AppError::InvalidBam {
        path: path.to_path_buf(),
        detail: "The first compressed member does not expose a valid BGZF header.".to_string(),
    })?;

    if block_size < header.len() {
        return Err(AppError::InvalidBam {
            path: path.to_path_buf(),
            detail: "BGZF block size is smaller than the header length.".to_string(),
        });
    }

    let mut member = vec![0_u8; block_size];
    member[..header.len()].copy_from_slice(&header);
    file.read_exact(&mut member[header.len()..])
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                AppError::TruncatedFile {
                    path: path.to_path_buf(),
                    detail: "BGZF block ended before the declared block size.".to_string(),
                }
            } else {
                AppError::from_io(path, error)
            }
        })?;

    Ok(Some(member))
}

fn bgzf_block_size(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 18 || !is_gzip_signature(bytes) || bytes[3] & 0x04 == 0 {
        return None;
    }

    let xlen = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
    if bytes.len() < 12 + xlen {
        return None;
    }

    let mut cursor = 12;
    let extra_end = 12 + xlen;

    while cursor + 4 <= extra_end {
        let subfield_len = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]) as usize;
        let payload_start = cursor + 4;
        let payload_end = payload_start + subfield_len;
        if payload_end > extra_end {
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

fn decompress_member(member: &[u8], path: &Path) -> Result<Vec<u8>, AppError> {
    let mut decoder = GzDecoder::new(member);
    let mut payload = Vec::new();
    decoder
        .read_to_end(&mut payload)
        .map_err(|error| AppError::InvalidBam {
            path: path.to_path_buf(),
            detail: format!("Unable to inflate the first BGZF member: {error}"),
        })?;
    Ok(payload)
}

#[cfg(test)]
pub mod test_support {
    use std::{fs, io::Write, path::PathBuf};

    use flate2::{Compression, GzBuilder};

    use super::BGZF_EOF_MARKER;

    pub fn build_bgzf_member(payload: &[u8]) -> Vec<u8> {
        let extra = [b'B', b'C', 2, 0, 0, 0];
        let mut encoder = GzBuilder::new()
            .extra(extra.as_slice())
            .write(Vec::new(), Compression::default());
        encoder
            .write_all(payload)
            .expect("bgzf fixture should compress");
        let mut member = encoder.finish().expect("bgzf fixture should finish");
        let bsize = (member.len() - 1) as u16;
        member[16..18].copy_from_slice(&bsize.to_le_bytes());
        member
    }

    pub fn build_bam_file() -> Vec<u8> {
        build_bam_file_with_header("", &[])
    }

    pub fn build_bam_file_with_header(header_text: &str, references: &[(&str, u32)]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&(header_text.len() as i32).to_le_bytes());
        payload.extend_from_slice(header_text.as_bytes());
        payload.extend_from_slice(&(references.len() as i32).to_le_bytes());

        for (name, length) in references {
            let mut nul_terminated_name = name.as_bytes().to_vec();
            nul_terminated_name.push(0);
            payload.extend_from_slice(&(nul_terminated_name.len() as i32).to_le_bytes());
            payload.extend_from_slice(&nul_terminated_name);
            payload.extend_from_slice(&(*length as i32).to_le_bytes());
        }

        let mut bytes = build_bgzf_member(&payload);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        bytes
    }

    pub fn write_temp_file(name: &str, suffix: &str, bytes: &[u8]) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("bamana-{name}-{}-{suffix}", std::process::id()));
        fs::write(&path, bytes).expect("test fixture should be written");
        path
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{BGZF_EOF_MARKER, first_member_starts_with_bam_magic, has_bgzf_eof, test_support};

    #[test]
    fn detects_bam_magic_in_first_bgzf_member() {
        let path = test_support::write_temp_file("verify", "bam", &test_support::build_bam_file());
        let result = first_member_starts_with_bam_magic(&path).expect("bam magic should be read");
        fs::remove_file(path).expect("fixture should be removed");
        assert!(result);
    }

    #[test]
    fn detects_canonical_bgzf_eof() {
        let path = test_support::write_temp_file("eof", "bam", &test_support::build_bam_file());
        let result = has_bgzf_eof(&path).expect("eof check should succeed");
        fs::remove_file(path).expect("fixture should be removed");
        assert!(result);
    }

    #[test]
    fn rejects_short_file_for_eof_check() {
        let path = test_support::write_temp_file("short", "bam", &BGZF_EOF_MARKER[..10]);
        let result = has_bgzf_eof(&path);
        fs::remove_file(path).expect("fixture should be removed");
        assert!(result.is_err());
    }
}
