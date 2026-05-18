pub mod block;
pub mod reader;
pub mod virtual_offset;
pub mod writer;

pub use block::{BGZF_EOF_MARKER, is_bgzf_header, is_gzip_signature};
pub use reader::{first_member_starts_with_bam_magic, has_bgzf_eof, read_bgzf_payloads};
pub use virtual_offset::{
    MAX_COMPRESSED_BLOCK_OFFSET, MAX_UNCOMPRESSED_BLOCK_OFFSET, VirtualOffset, VirtualOffsetError,
};
pub use writer::BgzfWriter;

#[cfg(test)]
pub mod test_support {
    use std::{fs, path::PathBuf};

    use super::{BGZF_EOF_MARKER, block::build_bgzf_member as build_native_bgzf_member};

    pub fn build_bgzf_member_for_test(payload: &[u8]) -> Vec<u8> {
        build_native_bgzf_member(payload).expect("bgzf fixture should compress")
    }

    pub fn build_bgzf_member(payload: &[u8]) -> Vec<u8> {
        build_bgzf_member_for_test(payload)
    }

    pub fn build_bam_file() -> Vec<u8> {
        build_bam_file_with_header("", &[])
    }

    pub fn build_bam_file_with_header(header_text: &str, references: &[(&str, u32)]) -> Vec<u8> {
        build_bam_file_with_header_and_records(header_text, references, &[])
    }

    pub fn build_bam_file_with_header_and_records(
        header_text: &str,
        references: &[(&str, u32)],
        records: &[Vec<u8>],
    ) -> Vec<u8> {
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

        for record in records {
            payload.extend_from_slice(record);
        }

        let mut bytes = build_bgzf_member(&payload);
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        bytes
    }

    pub fn build_light_record(ref_id: i32, pos: i32, read_name: &str, flags: u16) -> Vec<u8> {
        build_light_record_with_mate(ref_id, pos, read_name, flags, -1, -1)
    }

    pub fn build_light_record_with_mate(
        ref_id: i32,
        pos: i32,
        read_name: &str,
        flags: u16,
        next_ref_id: i32,
        next_pos: i32,
    ) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(read_name.as_bytes());
        variable.push(0);

        let l_read_name = variable.len() as u32;
        let block_size = 32 + variable.len();
        let bin_mq_nl = l_read_name;
        let flag_nc = (flags as u32) << 16;

        let mut record = Vec::with_capacity(4 + block_size);
        record.extend_from_slice(&(block_size as i32).to_le_bytes());
        record.extend_from_slice(&ref_id.to_le_bytes());
        record.extend_from_slice(&pos.to_le_bytes());
        record.extend_from_slice(&bin_mq_nl.to_le_bytes());
        record.extend_from_slice(&flag_nc.to_le_bytes());
        record.extend_from_slice(&0_i32.to_le_bytes());
        record.extend_from_slice(&next_ref_id.to_le_bytes());
        record.extend_from_slice(&next_pos.to_le_bytes());
        record.extend_from_slice(&0_i32.to_le_bytes());
        record.extend_from_slice(&variable);
        record
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
