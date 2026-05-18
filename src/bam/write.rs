use crate::bam::records::RecordLayout;

pub use crate::bgzf::BgzfWriter;

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
