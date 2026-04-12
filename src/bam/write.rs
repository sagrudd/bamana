use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use flate2::{Compression, GzBuilder};

use crate::{bam::records::RecordLayout, error::AppError, formats::bgzf::BGZF_EOF_MARKER};

const BGZF_MAX_BLOCK_SIZE: usize = 65_536;
const BGZF_TARGET_UNCOMPRESSED_BLOCK: usize = 64 * 1024 - 512;
const BGZF_BLOCK_REDUCTION_STEP: usize = 1024;

pub struct BgzfWriter {
    path: PathBuf,
    writer: BufWriter<File>,
    buffer: Vec<u8>,
}

impl BgzfWriter {
    pub fn create(path: &Path) -> Result<Self, AppError> {
        let file = File::create(path).map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        Ok(Self {
            path: path.to_path_buf(),
            writer: BufWriter::new(file),
            buffer: Vec::with_capacity(BGZF_TARGET_UNCOMPRESSED_BLOCK * 2),
        })
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), AppError> {
        self.buffer.extend_from_slice(bytes);
        while self.buffer.len() >= BGZF_TARGET_UNCOMPRESSED_BLOCK {
            self.flush_next_block(false)?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), AppError> {
        while !self.buffer.is_empty() {
            self.flush_next_block(true)?;
        }

        self.writer
            .write_all(&BGZF_EOF_MARKER)
            .map_err(|error| AppError::WriteError {
                path: self.path.clone(),
                message: error.to_string(),
            })?;
        self.writer.flush().map_err(|error| AppError::WriteError {
            path: self.path.clone(),
            message: error.to_string(),
        })?;
        Ok(())
    }

    fn flush_next_block(&mut self, allow_small_block: bool) -> Result<(), AppError> {
        let max_candidate = if allow_small_block {
            self.buffer.len()
        } else {
            self.buffer.len().min(BGZF_TARGET_UNCOMPRESSED_BLOCK)
        };
        let (member, consumed) =
            build_bgzf_member_fitting(&self.buffer[..max_candidate], &self.path)?;
        self.writer
            .write_all(&member)
            .map_err(|error| AppError::WriteError {
                path: self.path.clone(),
                message: error.to_string(),
            })?;
        self.buffer.drain(..consumed);
        Ok(())
    }
}

pub fn serialize_record_layout(record: &RecordLayout) -> Vec<u8> {
    let mut read_name = record.read_name.as_bytes().to_vec();
    read_name.push(0);

    let variable_len = read_name.len()
        + record.cigar_bytes.len()
        + record.sequence_bytes.len()
        + record.quality_bytes.len()
        + record.aux_bytes.len();
    let block_size = 32 + variable_len;

    let mut bytes = Vec::with_capacity(4 + block_size);
    bytes.extend_from_slice(&(block_size as i32).to_le_bytes());
    bytes.extend_from_slice(&record.ref_id.to_le_bytes());
    bytes.extend_from_slice(&record.pos.to_le_bytes());

    let bin_mq_nl = ((record.bin as u32) << 16)
        | ((record.mapping_quality as u32) << 8)
        | (read_name.len() as u32);
    bytes.extend_from_slice(&bin_mq_nl.to_le_bytes());

    let flag_nc = ((record.flags as u32) << 16) | (record.n_cigar_op as u32);
    bytes.extend_from_slice(&flag_nc.to_le_bytes());
    bytes.extend_from_slice(&(record.l_seq as i32).to_le_bytes());
    bytes.extend_from_slice(&record.next_ref_id.to_le_bytes());
    bytes.extend_from_slice(&record.next_pos.to_le_bytes());
    bytes.extend_from_slice(&record.tlen.to_le_bytes());
    bytes.extend_from_slice(&read_name);
    bytes.extend_from_slice(&record.cigar_bytes);
    bytes.extend_from_slice(&record.sequence_bytes);
    bytes.extend_from_slice(&record.quality_bytes);
    bytes.extend_from_slice(&record.aux_bytes);
    bytes
}

fn build_bgzf_member_fitting(payload: &[u8], path: &Path) -> Result<(Vec<u8>, usize), AppError> {
    let mut candidate_len = payload.len().min(BGZF_TARGET_UNCOMPRESSED_BLOCK);

    loop {
        let member =
            build_bgzf_member(&payload[..candidate_len]).map_err(|error| AppError::WriteError {
                path: path.to_path_buf(),
                message: error,
            })?;
        if member.len() <= BGZF_MAX_BLOCK_SIZE {
            return Ok((member, candidate_len));
        }

        if candidate_len <= BGZF_BLOCK_REDUCTION_STEP {
            return Err(AppError::WriteError {
                path: path.to_path_buf(),
                message: "Unable to fit BAM output bytes into a BGZF block.".to_string(),
            });
        }
        candidate_len -= BGZF_BLOCK_REDUCTION_STEP;
    }
}

fn build_bgzf_member(payload: &[u8]) -> Result<Vec<u8>, String> {
    let extra = [b'B', b'C', 2, 0, 0, 0];
    let mut encoder = GzBuilder::new()
        .extra(extra.as_slice())
        .write(Vec::new(), Compression::default());
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

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        bam::{header::parse_bam_header, reader::BamReader, records::read_next_record_layout},
        formats::bgzf::has_bgzf_eof,
    };

    use super::{BgzfWriter, serialize_record_layout};

    #[test]
    fn writer_emits_bgzf_stream_with_eof() {
        let path = std::env::temp_dir().join(format!("bamana-writer-{}.bam", std::process::id()));
        let mut writer = BgzfWriter::create(&path).expect("writer should create");
        writer
            .write_all(b"BAM\x01\x00\x00\x00\x00\x00\x00\x00")
            .expect("writer should accept bytes");
        writer.finish().expect("writer should finish");

        assert!(has_bgzf_eof(&path).expect("EOF check should succeed"));
        fs::remove_file(path).expect("fixture should be removable");
    }

    #[test]
    fn serializing_and_re_reading_a_record_round_trips() {
        let bytes = crate::formats::bgzf::test_support::build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[crate::formats::bgzf::test_support::build_light_record(
                0, 5, "read1", 0x10,
            )],
        );
        let source =
            crate::formats::bgzf::test_support::write_temp_file("roundtrip-source", "bam", &bytes);
        let _header = parse_bam_header(&source).expect("header should parse");
        let mut reader = BamReader::open(&source).expect("source should reopen");
        let _header = crate::bam::header::parse_bam_header_from_reader(&mut reader)
            .expect("header should parse from reader");
        let record = read_next_record_layout(&mut reader)
            .expect("record read should succeed")
            .expect("record should exist");

        let output = std::env::temp_dir().join(format!(
            "bamana-writer-roundtrip-{}.bam",
            std::process::id()
        ));
        let mut writer = BgzfWriter::create(&output).expect("writer should create");
        let header_payload = crate::bam::header::serialize_bam_header_payload(
            "@SQ\tSN:chr1\tLN:10\n",
            &[crate::bam::header::ReferenceRecord {
                name: "chr1".to_string(),
                length: 10,
                index: 0,
                header_fields: crate::bam::header::ReferenceHeaderFields::default(),
                text_header_length: Some(10),
            }],
        );
        writer
            .write_all(&header_payload)
            .expect("header should write");
        writer
            .write_all(&serialize_record_layout(&record))
            .expect("record should write");
        writer.finish().expect("writer should finish");

        let reparsed = parse_bam_header(&output).expect("output header should parse");
        assert_eq!(reparsed.header.references.len(), 1);
        fs::remove_file(source).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }
}
