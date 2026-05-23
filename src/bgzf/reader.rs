use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::Path,
};

use flate2::read::GzDecoder;

use crate::{
    bgzf::block::{BGZF_EOF_MARKER, bgzf_block_size},
    bgzf::virtual_offset::{VirtualOffset, VirtualOffsetError},
    error::AppError,
};

pub struct NativeBgzfReader {
    path: std::path::PathBuf,
    file: File,
    payload: Vec<u8>,
    offset: usize,
    eof: bool,
    current_block_start: u64,
    current_block_end: u64,
    has_current_block: bool,
}

impl NativeBgzfReader {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;

        Ok(Self {
            path: path.to_path_buf(),
            file,
            payload: Vec::new(),
            offset: 0,
            eof: false,
            current_block_start: 0,
            current_block_end: 0,
            has_current_block: false,
        })
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, AppError> {
        if buffer.is_empty() {
            return Ok(0);
        }

        while self.offset >= self.payload.len() {
            if self.eof {
                return Ok(0);
            }
            self.load_next_payload()?;
        }

        let available = self.payload.len() - self.offset;
        let count = available.min(buffer.len());
        buffer[..count].copy_from_slice(&self.payload[self.offset..self.offset + count]);
        self.offset += count;
        Ok(count)
    }

    pub fn virtual_offset(&self) -> Result<VirtualOffset, AppError> {
        if !self.has_current_block {
            return Ok(VirtualOffset::ZERO);
        }

        if self.offset >= self.payload.len() {
            return VirtualOffset::new(self.current_block_end, 0)
                .map_err(|error| virtual_offset_error(&self.path, error));
        }

        VirtualOffset::new(self.current_block_start, self.offset as u32)
            .map_err(|error| virtual_offset_error(&self.path, error))
    }

    pub fn seek_virtual_offset(&mut self, offset: VirtualOffset) -> Result<(), AppError> {
        self.file
            .seek(SeekFrom::Start(offset.compressed_block_offset()))
            .map_err(|error| AppError::from_io(&self.path, error))?;
        self.payload.clear();
        self.offset = 0;
        self.eof = false;
        self.current_block_start = offset.compressed_block_offset();
        self.current_block_end = offset.compressed_block_offset();
        self.has_current_block = false;

        let member =
            read_bgzf_member(&mut self.file, &self.path)?.ok_or_else(|| AppError::InvalidBam {
                path: self.path.clone(),
                detail: format!(
                    "BGZF virtual offset {} points beyond the end of the compressed stream.",
                    offset.packed()
                ),
            })?;

        if member.bytes == BGZF_EOF_MARKER {
            return Err(AppError::InvalidBam {
                path: self.path.clone(),
                detail: "BGZF virtual offset points at the EOF marker, not a data member."
                    .to_string(),
            });
        }

        let payload = decompress_member(&member.bytes, &self.path)?;
        let in_block_offset = usize::from(offset.uncompressed_block_offset());
        if in_block_offset > payload.len() {
            return Err(AppError::InvalidBam {
                path: self.path.clone(),
                detail: format!(
                    "BGZF virtual offset in-block component {in_block_offset} exceeds inflated member length {}.",
                    payload.len()
                ),
            });
        }

        self.current_block_start = member.compressed_offset;
        self.current_block_end = member.compressed_offset + member.bytes.len() as u64;
        self.has_current_block = true;
        self.payload = payload;
        self.offset = in_block_offset;
        Ok(())
    }

    fn load_next_payload(&mut self) -> Result<(), AppError> {
        loop {
            let Some(member) = read_bgzf_member(&mut self.file, &self.path)? else {
                self.eof = true;
                self.payload.clear();
                self.offset = 0;
                self.has_current_block = false;
                return Ok(());
            };

            if member.bytes == BGZF_EOF_MARKER {
                self.eof = true;
                self.payload.clear();
                self.offset = 0;
                self.has_current_block = false;
                return Ok(());
            }

            self.current_block_start = member.compressed_offset;
            self.current_block_end = member.compressed_offset + member.bytes.len() as u64;
            self.has_current_block = true;
            self.payload = decompress_member(&member.bytes, &self.path)?;
            self.offset = 0;

            if !self.payload.is_empty() {
                return Ok(());
            }
        }
    }
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

pub fn read_bgzf_payloads(path: &Path) -> Result<Vec<Vec<u8>>, AppError> {
    let mut file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let mut payloads = Vec::new();

    while let Some(member) = read_bgzf_member(&mut file, path)? {
        if member.bytes == BGZF_EOF_MARKER {
            break;
        }
        payloads.push(decompress_member(&member.bytes, path)?);
    }

    Ok(payloads)
}

fn virtual_offset_error(path: &Path, error: VirtualOffsetError) -> AppError {
    AppError::InvalidBam {
        path: path.to_path_buf(),
        detail: format!("BGZF virtual offset could not be represented: {error}"),
    }
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
    decompress_member(&member.bytes, path)
}

struct BgzfMember {
    compressed_offset: u64,
    bytes: Vec<u8>,
}

fn read_bgzf_member(file: &mut File, path: &Path) -> Result<Option<BgzfMember>, AppError> {
    let compressed_offset = file
        .stream_position()
        .map_err(|error| AppError::from_io(path, error))?;
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

    Ok(Some(BgzfMember {
        compressed_offset,
        bytes: member,
    }))
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

    use crate::{
        bgzf::{block::build_bgzf_member, virtual_offset::VirtualOffset},
        error::AppError,
    };

    use super::{
        BGZF_EOF_MARKER, NativeBgzfReader, decompress_member, first_member_starts_with_bam_magic,
        has_bgzf_eof, read_bgzf_member, read_bgzf_payloads, read_first_bgzf_member_payload,
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
            decompress_member(&first_member.bytes, &path).expect("first member should inflate");
        let second_payload =
            decompress_member(&second_member.bytes, &path).expect("second member should inflate");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(first_member.compressed_offset, 0);
        assert_eq!(second_member.compressed_offset, first.len() as u64);
        assert_eq!(first_payload, b"first member");
        assert_eq!(second_payload, b"second member");
    }

    #[test]
    fn native_reader_reports_virtual_offsets_across_members() {
        let first = member(b"abcdefghij");
        let second = member(b"klmnopqrst");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first);
        bytes.extend_from_slice(&second);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("virtual-offsets", &bytes);

        let mut reader = NativeBgzfReader::open(&path).expect("reader should open");
        assert_eq!(
            reader
                .virtual_offset()
                .expect("virtual offset should be available")
                .packed(),
            0
        );

        let mut buffer = [0_u8; 4];
        assert_eq!(reader.read(&mut buffer).expect("first read should work"), 4);
        assert_eq!(&buffer, b"abcd");
        let after_first_read = reader
            .virtual_offset()
            .expect("virtual offset should be available");
        assert_eq!(after_first_read.compressed_block_offset(), 0);
        assert_eq!(after_first_read.uncompressed_block_offset(), 4);

        let mut rest_of_first = [0_u8; 6];
        assert_eq!(
            reader
                .read(&mut rest_of_first)
                .expect("second read should work"),
            6
        );
        assert_eq!(&rest_of_first, b"efghij");
        let at_block_boundary = reader
            .virtual_offset()
            .expect("virtual offset should be available");
        assert_eq!(
            at_block_boundary.compressed_block_offset(),
            first.len() as u64
        );
        assert_eq!(at_block_boundary.uncompressed_block_offset(), 0);

        let mut next = [0_u8; 2];
        assert_eq!(reader.read(&mut next).expect("third read should work"), 2);
        assert_eq!(&next, b"kl");
        let in_second_block = reader
            .virtual_offset()
            .expect("virtual offset should be available");
        assert_eq!(
            in_second_block.compressed_block_offset(),
            first.len() as u64
        );
        assert_eq!(in_second_block.uncompressed_block_offset(), 2);

        fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn native_reader_seeks_to_virtual_offset_inside_member() {
        let first = member(b"abcdefghij");
        let second = member(b"klmnopqrst");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&first);
        bytes.extend_from_slice(&second);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("seek-virtual-offset", &bytes);
        let mut reader = NativeBgzfReader::open(&path).expect("reader should open");
        let offset = VirtualOffset::new(first.len() as u64, 3).expect("offset should fit");

        reader
            .seek_virtual_offset(offset)
            .expect("seek should succeed");
        let mut buffer = [0_u8; 4];
        assert_eq!(reader.read(&mut buffer).expect("read should work"), 4);

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(&buffer, b"nopq");
    }

    #[test]
    fn native_reader_rejects_virtual_offset_beyond_inflated_member() {
        let mut bytes = member(b"abc");
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        let path = write_temp_file("seek-invalid-in-block", &bytes);
        let mut reader = NativeBgzfReader::open(&path).expect("reader should open");
        let offset = VirtualOffset::new(0, 4).expect("offset should fit");

        let error = reader
            .seek_virtual_offset(offset)
            .expect_err("invalid in-block offset should fail");

        fs::remove_file(path).expect("fixture should be removed");
        assert_invalid_bam_detail_contains(error, "exceeds inflated member length");
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
