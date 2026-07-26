use std::io::Write;

use flate2::{Compression, GzBuilder};

pub const BGZF_EOF_MARKER: [u8; 28] = [
    31, 139, 8, 4, 0, 0, 0, 0, 0, 255, 6, 0, 66, 67, 2, 0, 27, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub(crate) const BGZF_MAX_BLOCK_SIZE: usize = 65_536;
// Full independent compression jobs use a fixed payload. If a requested
// compressed representation expands beyond the BGZF member limit, the member
// builder deterministically falls back to stored DEFLATE for that payload.
pub(crate) const BGZF_TARGET_UNCOMPRESSED_BLOCK: usize = 64_000;
pub(crate) const BGZF_BLOCK_REDUCTION_STEP: usize = 1024;

pub fn is_gzip_signature(bytes: &[u8]) -> bool {
    bytes.len() >= 3 && bytes[0] == 0x1f && bytes[1] == 0x8b && bytes[2] == 0x08
}

pub fn is_bgzf_header(bytes: &[u8]) -> bool {
    bgzf_block_size(bytes).is_some()
}

pub(crate) fn bgzf_block_size(bytes: &[u8]) -> Option<usize> {
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

pub(crate) fn build_bgzf_member_fitting_with_level(
    payload: &[u8],
    compression_level: u32,
) -> Result<(Vec<u8>, usize), String> {
    if compression_level > 9 {
        return Err("BGZF compression level must be between 0 and 9.".to_string());
    }
    let mut candidate_len = payload.len().min(BGZF_TARGET_UNCOMPRESSED_BLOCK);

    loop {
        let member = build_bgzf_member_with_level(&payload[..candidate_len], compression_level)?;
        if member.len() <= BGZF_MAX_BLOCK_SIZE {
            return Ok((member, candidate_len));
        }
        if compression_level != 0 {
            let stored = build_bgzf_member_with_level(&payload[..candidate_len], 0)?;
            if stored.len() <= BGZF_MAX_BLOCK_SIZE {
                return Ok((stored, candidate_len));
            }
        }

        if candidate_len <= BGZF_BLOCK_REDUCTION_STEP {
            return Err("Unable to fit BAM output bytes into a BGZF block.".to_string());
        }
        candidate_len -= BGZF_BLOCK_REDUCTION_STEP;
    }
}

#[cfg(test)]
pub(crate) fn build_bgzf_member(payload: &[u8]) -> Result<Vec<u8>, String> {
    build_bgzf_member_with_level(payload, 6)
}

pub(crate) fn build_bgzf_member_with_level(
    payload: &[u8],
    compression_level: u32,
) -> Result<Vec<u8>, String> {
    let extra = [b'B', b'C', 2, 0, 0, 0];
    let mut encoder = GzBuilder::new()
        .extra(extra.as_slice())
        .write(Vec::new(), Compression::new(compression_level));
    encoder
        .write_all(payload)
        .map_err(|error| format!("BGZF member compression failed: {error}"))?;
    let mut member = encoder
        .finish()
        .map_err(|error| format!("BGZF member finalization failed: {error}"))?;
    if member.len() > BGZF_MAX_BLOCK_SIZE {
        return Ok(member);
    }

    let bsize = (member.len() - 1) as u16;
    if member.len() < 18 {
        return Err("Compressed BGZF member was shorter than the expected header.".to_string());
    }
    member[16..18].copy_from_slice(&bsize.to_le_bytes());
    Ok(member)
}
