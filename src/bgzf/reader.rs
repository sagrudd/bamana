use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
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

pub fn read_bgzf_payloads(path: &Path) -> Result<Vec<Vec<u8>>, AppError> {
    let mut file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let mut payloads = Vec::new();

    while let Some(member) = read_bgzf_member(&mut file, path)? {
        if member == BGZF_EOF_MARKER {
            break;
        }
        payloads.push(decompress_member(&member, path)?);
    }

    Ok(payloads)
}

fn read_first_bgzf_payload(path: &Path) -> Result<Vec<u8>, AppError> {
    let member = read_bgzf_payloads(path)?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::TruncatedFile {
            path: path.to_path_buf(),
            detail: "File did not contain a readable BGZF member before EOF.".to_string(),
        })?;

    Ok(member)
}

#[cfg(test)]
fn read_first_bgzf_member_payload(path: &Path) -> Result<Vec<u8>, AppError> {
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

    file.read_exact(&mut fixed_header[1..]).map_err(|error| {
        if error.kind() == ErrorKind::UnexpectedEof {
            AppError::TruncatedFile {
                path: path.to_path_buf(),
                detail: "BGZF fixed header ended before 12 bytes.".to_string(),
            }
        } else {
            AppError::from_io(path, error)
        }
    })?;

    let xlen = u16::from_le_bytes([fixed_header[10], fixed_header[11]]) as usize;
    let mut extra = vec![0_u8; xlen];
    file.read_exact(&mut extra).map_err(|error| {
        if error.kind() == ErrorKind::UnexpectedEof {
            AppError::TruncatedFile {
                path: path.to_path_buf(),
                detail: "BGZF extra header ended before the declared XLEN bytes.".to_string(),
            }
        } else {
            AppError::from_io(path, error)
        }
    })?;

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
            if error.kind() == ErrorKind::UnexpectedEof {
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

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{bgzf::block::build_bgzf_member, error::AppError};

    use super::{
        BGZF_EOF_MARKER, decompress_member, first_member_starts_with_bam_magic, has_bgzf_eof,
        read_bgzf_member, read_bgzf_payloads, read_first_bgzf_member_payload,
        read_first_bgzf_payload,
    };

    fn write_temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bamana-bgzf-reader-{name}-{}-{nonce}.bam",
            std::process::id()
        ));
        fs::write(&path, bytes).expect("reader fixture should be written");
        path
    }

    fn member(payload: &[u8]) -> Vec<u8> {
        build_bgzf_member(payload).expect("fixture member should compress")
    }

    fn assert_truncated_detail(error: AppError, expected: &str) {
        match error {
            AppError::TruncatedFile { detail, .. } => assert_eq!(detail, expected),
            other => panic!("expected truncated file error, got {other:?}"),
        }
    }

    fn assert_invalid_bam_detail_contains(error: AppError, expected: &str) {
        match error {
            AppError::InvalidBam { detail, .. } => assert!(
                detail.contains(expected),
                "expected detail to contain {expected:?}, got {detail:?}"
            ),
            other => panic!("expected invalid BAM error, got {other:?}"),
        }
    }

    #[test]
    fn reads_valid_single_block_bgzf_stream() {
        let path = write_temp_file("single-block", &member(b"single-block payload"));

        let payload = read_first_bgzf_payload(&path).expect("payload should inflate");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(payload, b"single-block payload");
    }

    #[test]
    fn reads_declared_members_from_multi_block_stream() {
        let first = member(b"first member");
        let second = member(b"second member");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first);
        bytes.extend_from_slice(&second);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);

        let path = write_temp_file("multi-block", &bytes);
        let mut file = File::open(&path).expect("fixture should open");
        let first_member = read_bgzf_member(&mut file, &path)
            .expect("first member should read")
            .expect("first member should exist");
        let second_member = read_bgzf_member(&mut file, &path)
            .expect("second member should read")
            .expect("second member should exist");

        let first_payload =
            decompress_member(&first_member, &path).expect("first member should inflate");
        let second_payload =
            decompress_member(&second_member, &path).expect("second member should inflate");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(first_payload, b"first member");
        assert_eq!(second_payload, b"second member");
    }

    #[test]
    fn reports_truncated_fixed_header() {
        let truncated = &member(b"payload")[..8];
        let path = write_temp_file("truncated-fixed-header", truncated);

        let error =
            read_first_bgzf_member_payload(&path).expect_err("fixed header should be truncated");

        fs::remove_file(path).expect("fixture should be removed");
        assert_truncated_detail(error, "BGZF fixed header ended before 12 bytes.");
    }

    #[test]
    fn reports_truncated_extra_header() {
        let bytes = &member(b"payload")[..14];
        let path = write_temp_file("truncated-extra-header", bytes);

        let error =
            read_first_bgzf_member_payload(&path).expect_err("extra header should be truncated");

        fs::remove_file(path).expect("fixture should be removed");
        assert_truncated_detail(
            error,
            "BGZF extra header ended before the declared XLEN bytes.",
        );
    }

    #[test]
    fn reports_truncated_compressed_payload() {
        let mut bytes = member(b"payload");
        bytes.truncate(bytes.len() - 5);
        let path = write_temp_file("truncated-payload", &bytes);

        let error = read_first_bgzf_member_payload(&path).expect_err("payload should be truncated");

        fs::remove_file(path).expect("fixture should be removed");
        assert_truncated_detail(error, "BGZF block ended before the declared block size.");
    }

    #[test]
    fn rejects_invalid_crc_or_size_metadata() {
        let mut bytes = member(b"payload");
        let crc_start = bytes.len() - 8;
        bytes[crc_start] ^= 0xff;
        let path = write_temp_file("invalid-crc", &bytes);

        let error =
            read_first_bgzf_member_payload(&path).expect_err("crc mismatch should fail inflate");

        fs::remove_file(path).expect("fixture should be removed");
        assert_invalid_bam_detail_contains(error, "Unable to inflate the first BGZF member");
    }

    #[test]
    fn reads_bam_magic_from_first_inflated_member() {
        let mut bytes = member(b"BAM\x01payload");
        bytes.extend_from_slice(&member(b"second member is not inspected"));
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("bam-magic", &bytes);

        let starts_with_bam_magic =
            first_member_starts_with_bam_magic(&path).expect("first member should inflate");

        fs::remove_file(path).expect("fixture should be removed");
        assert!(starts_with_bam_magic);
    }

    #[test]
    fn detects_eof_independently_from_bam_magic() {
        let mut bytes = member(b"not a BAM payload");
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("eof-without-bam", &bytes);

        let eof_present = has_bgzf_eof(&path).expect("eof check should succeed");
        let starts_with_bam_magic =
            first_member_starts_with_bam_magic(&path).expect("first member should inflate");

        fs::remove_file(path).expect("fixture should be removed");
        assert!(eof_present);
        assert!(!starts_with_bam_magic);
    }

    #[test]
    fn detects_eof_independently_from_payload_validity() {
        let mut bytes = member(b"payload");
        let crc_start = bytes.len() - 8;
        bytes[crc_start] ^= 0xff;
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("eof-with-invalid-payload", &bytes);

        let eof_present = has_bgzf_eof(&path).expect("eof check should succeed");
        let error =
            first_member_starts_with_bam_magic(&path).expect_err("invalid payload should fail");

        fs::remove_file(path).expect("fixture should be removed");
        assert!(eof_present);
        assert_invalid_bam_detail_contains(error, "Unable to inflate the first BGZF member");
    }

    #[test]
    fn all_payload_reader_stops_at_canonical_eof_marker() {
        let mut bytes = member(b"payload before eof");
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("all-payloads", &bytes);

        let payloads = read_bgzf_payloads(&path).expect("payloads should read");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(payloads, vec![b"payload before eof".to_vec()]);
    }
}
