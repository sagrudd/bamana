use std::collections::HashSet;

use clap::ValueEnum;
use serde::Serialize;

use crate::{
    bam::{
        header::{HeaderPayload, parse_bam_header_from_reader},
        reader::BamReader,
        records::{RecordLayout, read_next_record_layout},
        tags::serialize_filtered_aux,
    },
    error::AppError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[value(rename_all = "kebab-case")]
pub enum ChecksumMode {
    RawRecordOrder,
    CanonicalRecordOrder,
    Header,
    Payload,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumAlgorithm {
    Sha256,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ChecksumFilters {
    pub only_primary: bool,
    pub mapped_only: bool,
}

#[derive(Debug, Serialize)]
pub struct ChecksumPayload {
    pub format: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<ChecksumAlgorithm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<ChecksumResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChecksumResult {
    pub mode: ChecksumMode,
    pub digest: String,
    pub records_hashed: u64,
    pub order_sensitive: bool,
    pub header_included: bool,
    pub filters: ChecksumFilters,
    pub excluded_tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ChecksumOptions {
    pub mode: ChecksumMode,
    pub algorithm: ChecksumAlgorithm,
    pub include_header: bool,
    pub excluded_tags: HashSet<[u8; 2]>,
    pub excluded_tag_strings: Vec<String>,
    pub filters: ChecksumFilters,
}

pub fn compute_checksums(
    path: &std::path::Path,
    options: &ChecksumOptions,
) -> Result<ChecksumPayload, AppError> {
    let mut reader = BamReader::open(path)?;
    let header = parse_bam_header_from_reader(&mut reader)?;
    let header_bytes = serialize_header(&header);

    let requested_modes = requested_modes(options.mode);
    let needs_record_scan = requested_modes
        .iter()
        .any(|mode| !matches!(mode, ChecksumMode::Header));

    let header_result = requested_modes
        .iter()
        .any(|mode| matches!(mode, ChecksumMode::Header))
        .then(|| checksum_result_for_header(&header_bytes, options));

    let record_results = if needs_record_scan {
        Some(scan_record_checksums(&mut reader, &header_bytes, options)?)
    } else {
        None
    };

    let mut results = Vec::new();
    if let Some(result) = header_result {
        results.push(result);
    }
    if let Some(mut record_results) = record_results {
        results.append(&mut record_results);
    }

    Ok(ChecksumPayload {
        format: "BAM",
        algorithm: Some(options.algorithm),
        results: Some(results),
        semantic_note: Some(build_semantic_note(options)),
    })
}

fn scan_record_checksums(
    reader: &mut BamReader,
    header_bytes: &[u8],
    options: &ChecksumOptions,
) -> Result<Vec<ChecksumResult>, AppError> {
    let mut raw_hasher =
        wants_mode(options.mode, ChecksumMode::RawRecordOrder).then(Sha256Hasher::new);
    let mut payload_hasher =
        wants_mode(options.mode, ChecksumMode::Payload).then(Sha256Hasher::new);
    let mut canonical_record_digests =
        if wants_mode(options.mode, ChecksumMode::CanonicalRecordOrder) {
            Some(Vec::new())
        } else {
            None
        };

    if options.include_header {
        if let Some(hasher) = payload_hasher.as_mut() {
            update_with_chunk(hasher, header_bytes);
        }
    }

    let mut records_hashed_raw = 0_u64;
    let mut records_hashed_payload = 0_u64;
    let mut records_hashed_canonical = 0_u64;

    loop {
        let Some(record) = read_next_record_layout(reader)? else {
            break;
        };

        if !record_included(&record, options.filters) {
            continue;
        }

        let serialized = serialize_record(&record, &options.excluded_tags).map_err(|detail| {
            AppError::ChecksumUncertainty {
                path: reader.path().to_path_buf(),
                detail,
            }
        })?;

        if let Some(hasher) = raw_hasher.as_mut() {
            update_with_chunk(hasher, &serialized);
            records_hashed_raw += 1;
        }
        if let Some(hasher) = payload_hasher.as_mut() {
            update_with_chunk(hasher, &serialized);
            records_hashed_payload += 1;
        }
        if let Some(digests) = canonical_record_digests.as_mut() {
            let digest = Sha256Hasher::digest(&frame_chunk(&serialized));
            digests.push(digest.to_vec());
            records_hashed_canonical += 1;
        }
    }

    let mut results = Vec::new();
    if let Some(hasher) = raw_hasher {
        results.push(ChecksumResult {
            mode: ChecksumMode::RawRecordOrder,
            digest: hex_digest(&hasher.finalize()),
            records_hashed: records_hashed_raw,
            order_sensitive: true,
            header_included: false,
            filters: options.filters,
            excluded_tags: options.excluded_tag_strings.clone(),
        });
    }
    if let Some(digests) = canonical_record_digests {
        let mut sorted = digests;
        sorted.sort_unstable();
        let mut hasher = Sha256Hasher::new();
        for digest in &sorted {
            update_with_chunk(&mut hasher, digest);
        }
        results.push(ChecksumResult {
            mode: ChecksumMode::CanonicalRecordOrder,
            digest: hex_digest(&hasher.finalize()),
            records_hashed: records_hashed_canonical,
            order_sensitive: false,
            header_included: false,
            filters: options.filters,
            excluded_tags: options.excluded_tag_strings.clone(),
        });
    }
    if let Some(hasher) = payload_hasher {
        results.push(ChecksumResult {
            mode: ChecksumMode::Payload,
            digest: hex_digest(&hasher.finalize()),
            records_hashed: records_hashed_payload,
            order_sensitive: true,
            header_included: options.include_header,
            filters: options.filters,
            excluded_tags: options.excluded_tag_strings.clone(),
        });
    }

    Ok(results)
}

fn checksum_result_for_header(header_bytes: &[u8], options: &ChecksumOptions) -> ChecksumResult {
    let digest = Sha256Hasher::digest(&frame_chunk(header_bytes));
    ChecksumResult {
        mode: ChecksumMode::Header,
        digest: hex_digest(&digest),
        records_hashed: 0,
        order_sensitive: true,
        header_included: true,
        filters: options.filters,
        excluded_tags: Vec::new(),
    }
}

fn requested_modes(mode: ChecksumMode) -> Vec<ChecksumMode> {
    match mode {
        ChecksumMode::All => vec![
            ChecksumMode::RawRecordOrder,
            ChecksumMode::CanonicalRecordOrder,
            ChecksumMode::Header,
            ChecksumMode::Payload,
        ],
        mode => vec![mode],
    }
}

fn wants_mode(requested: ChecksumMode, candidate: ChecksumMode) -> bool {
    requested == ChecksumMode::All || requested == candidate
}

fn record_included(record: &RecordLayout, filters: ChecksumFilters) -> bool {
    if filters.only_primary && (record.flags & 0x100 != 0 || record.flags & 0x800 != 0) {
        return false;
    }
    if filters.mapped_only && (record.flags & 0x4 != 0 || record.ref_id < 0) {
        return false;
    }
    true
}

fn serialize_record(
    record: &RecordLayout,
    excluded_tags: &HashSet<[u8; 2]>,
) -> Result<Vec<u8>, String> {
    let filtered_aux = serialize_filtered_aux(&record.aux_bytes, excluded_tags)?;

    let mut bytes = Vec::new();
    write_len_prefixed(&mut bytes, record.read_name.as_bytes());
    bytes.extend_from_slice(&record.flags.to_le_bytes());
    bytes.extend_from_slice(&record.ref_id.to_le_bytes());
    bytes.extend_from_slice(&record.pos.to_le_bytes());
    bytes.push(record.mapping_quality);
    bytes.extend_from_slice(&(record.n_cigar_op as u32).to_le_bytes());
    bytes.extend_from_slice(&(record.l_seq as u32).to_le_bytes());
    bytes.extend_from_slice(&record.next_ref_id.to_le_bytes());
    bytes.extend_from_slice(&record.next_pos.to_le_bytes());
    bytes.extend_from_slice(&record.tlen.to_le_bytes());
    write_len_prefixed(&mut bytes, &record.cigar_bytes);
    write_len_prefixed(&mut bytes, &record.sequence_bytes);
    write_len_prefixed(&mut bytes, &record.quality_bytes);
    write_len_prefixed(&mut bytes, &filtered_aux);
    Ok(bytes)
}

fn serialize_header(header: &HeaderPayload) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_len_prefixed(&mut bytes, header.header.raw_header_text.as_bytes());
    bytes.extend_from_slice(&(header.header.references.len() as u32).to_le_bytes());
    for reference in &header.header.references {
        write_len_prefixed(&mut bytes, reference.name.as_bytes());
        bytes.extend_from_slice(&reference.length.to_le_bytes());
    }
    bytes
}

fn write_len_prefixed(target: &mut Vec<u8>, bytes: &[u8]) {
    target.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    target.extend_from_slice(bytes);
}

fn update_with_chunk(hasher: &mut Sha256Hasher, bytes: &[u8]) {
    hasher.update(frame_chunk(bytes));
}

fn frame_chunk(bytes: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(8 + bytes.len());
    framed.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    framed.extend_from_slice(bytes);
    framed
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn build_semantic_note(options: &ChecksumOptions) -> String {
    match options.mode {
        ChecksumMode::RawRecordOrder => {
            "This checksum is computed over a stable per-record serialization in encounter order and is sensitive to record ordering.".to_string()
        }
        ChecksumMode::CanonicalRecordOrder => {
            "This checksum is intended for content comparison across BAM files whose alignment record order may differ. It hashes per-record canonical serializations, sorts the per-record digests, and hashes the sorted digest list; duplicate records contribute multiplicity.".to_string()
        }
        ChecksumMode::Header => {
            "This checksum covers header text bytes plus the binary reference dictionary serialized in order.".to_string()
        }
        ChecksumMode::Payload => {
            if options.include_header {
                "This checksum is computed over the encounter-order canonical record payload with the deterministic header serialization prefixed to the stream.".to_string()
            } else {
                "This checksum is computed over the encounter-order canonical record payload without header content.".to_string()
            }
        }
        ChecksumMode::All => {
            "Returned checksum domains include both order-sensitive and order-insensitive record checksums plus a deterministic header checksum. Canonical mode collects per-record digests in memory and sorts them before final hashing in this slice.".to_string()
        }
    }
}

struct Sha256Hasher {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    len_bits: u64,
}

impl Sha256Hasher {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            len_bits: 0,
        }
    }

    fn digest(bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Self::new();
        hasher.update(bytes);
        hasher.finalize()
    }

    fn update(&mut self, bytes: impl AsRef<[u8]>) {
        let mut input = bytes.as_ref();
        self.len_bits = self.len_bits.wrapping_add((input.len() as u64) * 8);

        if self.buffer_len > 0 {
            let needed = 64 - self.buffer_len;
            let take = needed.min(input.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&input[..take]);
            self.buffer_len += take;
            input = &input[take..];

            if self.buffer_len == 64 {
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }

        while input.len() >= 64 {
            let block: &[u8; 64] = input[..64]
                .try_into()
                .expect("64-byte slice should convert to a block");
            self.process_block(block);
            input = &input[64..];
        }

        if !input.is_empty() {
            self.buffer[..input.len()].copy_from_slice(input);
            self.buffer_len = input.len();
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            let block = self.buffer;
            self.process_block(&block);
            self.buffer_len = 0;
        }

        self.buffer[self.buffer_len..56].fill(0);
        self.buffer[56..64].copy_from_slice(&self.len_bits.to_be_bytes());
        let block = self.buffer;
        self.process_block(&block);

        let mut output = [0_u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            output[index * 4..(index + 1) * 4].copy_from_slice(&word.to_be_bytes());
        }
        output
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut w = [0_u32; 64];
        for (index, chunk) in block.chunks_exact(4).take(16).enumerate() {
            w[index] = u32::from_be_bytes(
                chunk
                    .try_into()
                    .expect("4-byte slice should convert to a word"),
            );
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, fs};

    use crate::formats::bgzf::test_support::{
        build_bam_file_with_header_and_records, write_temp_file,
    };

    use super::{
        ChecksumAlgorithm, ChecksumFilters, ChecksumMode, ChecksumOptions, Sha256Hasher,
        compute_checksums, hex_digest,
    };

    fn build_record(ref_id: i32, pos: i32, flags: u16, read_name: &str, aux: &[u8]) -> Vec<u8> {
        let mut variable = Vec::new();
        variable.extend_from_slice(read_name.as_bytes());
        variable.push(0);
        variable.extend_from_slice(aux);

        let l_read_name = read_name.len() as u32 + 1;
        let bin_mq_nl = l_read_name;
        let flag_nc = (flags as u32) << 16;
        let block_size = 32 + variable.len();

        let mut record = Vec::new();
        record.extend_from_slice(&(block_size as i32).to_le_bytes());
        record.extend_from_slice(&ref_id.to_le_bytes());
        record.extend_from_slice(&pos.to_le_bytes());
        record.extend_from_slice(&bin_mq_nl.to_le_bytes());
        record.extend_from_slice(&flag_nc.to_le_bytes());
        record.extend_from_slice(&0_i32.to_le_bytes());
        record.extend_from_slice(&(-1_i32).to_le_bytes());
        record.extend_from_slice(&(-1_i32).to_le_bytes());
        record.extend_from_slice(&0_i32.to_le_bytes());
        record.extend_from_slice(&variable);
        record
    }

    fn default_options(mode: ChecksumMode) -> ChecksumOptions {
        ChecksumOptions {
            mode,
            algorithm: ChecksumAlgorithm::Sha256,
            include_header: false,
            excluded_tags: HashSet::new(),
            excluded_tag_strings: Vec::new(),
            filters: ChecksumFilters {
                only_primary: false,
                mapped_only: false,
            },
        }
    }

    #[test]
    fn canonical_checksum_is_order_insensitive() {
        let record_a = build_record(0, 1, 0, "read1", b"NMi\x01\0\0\0");
        let record_b = build_record(0, 2, 0, "read2", b"NMi\x02\0\0\0");
        let bam_a = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record_a.clone(), record_b.clone()],
        );
        let bam_b = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record_b, record_a],
        );
        let path_a = write_temp_file("checksum-canon-a", "bam", &bam_a);
        let path_b = write_temp_file("checksum-canon-b", "bam", &bam_b);

        let result_a = compute_checksums(
            &path_a,
            &default_options(ChecksumMode::CanonicalRecordOrder),
        )
        .expect("checksum should succeed");
        let result_b = compute_checksums(
            &path_b,
            &default_options(ChecksumMode::CanonicalRecordOrder),
        )
        .expect("checksum should succeed");

        fs::remove_file(path_a).expect("fixture should be removable");
        fs::remove_file(path_b).expect("fixture should be removable");

        assert_eq!(
            result_a.results.as_ref().unwrap()[0].digest,
            result_b.results.as_ref().unwrap()[0].digest
        );
    }

    #[test]
    fn raw_checksum_is_order_sensitive() {
        let record_a = build_record(0, 1, 0, "read1", b"");
        let record_b = build_record(0, 2, 0, "read2", b"");
        let bam_a = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record_a.clone(), record_b.clone()],
        );
        let bam_b = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record_b, record_a],
        );
        let path_a = write_temp_file("checksum-raw-a", "bam", &bam_a);
        let path_b = write_temp_file("checksum-raw-b", "bam", &bam_b);

        let result_a = compute_checksums(&path_a, &default_options(ChecksumMode::RawRecordOrder))
            .expect("checksum should succeed");
        let result_b = compute_checksums(&path_b, &default_options(ChecksumMode::RawRecordOrder))
            .expect("checksum should succeed");

        fs::remove_file(path_a).expect("fixture should be removable");
        fs::remove_file(path_b).expect("fixture should be removable");

        assert_ne!(
            result_a.results.as_ref().unwrap()[0].digest,
            result_b.results.as_ref().unwrap()[0].digest
        );
    }

    #[test]
    fn excluded_tags_affect_canonical_equivalence() {
        let record_a = build_record(0, 1, 0, "read1", b"NMi\x01\0\0\0");
        let record_b = build_record(0, 1, 0, "read1", b"NMi\x02\0\0\0");
        let bam_a = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record_a],
        );
        let bam_b = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[record_b],
        );
        let path_a = write_temp_file("checksum-excl-a", "bam", &bam_a);
        let path_b = write_temp_file("checksum-excl-b", "bam", &bam_b);

        let mut options = default_options(ChecksumMode::CanonicalRecordOrder);
        options.excluded_tags.insert(*b"NM");
        options.excluded_tag_strings.push("NM".to_string());

        let result_a = compute_checksums(&path_a, &options).expect("checksum should succeed");
        let result_b = compute_checksums(&path_b, &options).expect("checksum should succeed");

        fs::remove_file(path_a).expect("fixture should be removable");
        fs::remove_file(path_b).expect("fixture should be removable");

        assert_eq!(
            result_a.results.as_ref().unwrap()[0].digest,
            result_b.results.as_ref().unwrap()[0].digest
        );
    }

    #[test]
    fn sha256_matches_known_vector() {
        let digest = Sha256Hasher::digest(b"abc");
        assert_eq!(
            hex_digest(&digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
