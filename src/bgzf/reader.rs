use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use flate2::read::GzDecoder;

use crate::{
    bgzf::block::{BGZF_EOF_MARKER, bgzf_block_size},
    error::AppError,
};

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
