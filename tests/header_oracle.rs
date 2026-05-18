use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bamana::{
    bam::header::{parse_bam_header, serialize_bam_header_payload},
    bgzf::BgzfWriter,
};

// This integration test intentionally uses noodles as a test-only oracle.
// Production header and verify paths must not import or call noodles.
use noodles_bam as bam;

#[test]
fn native_header_matches_noodles_oracle_for_valid_reference_dictionary() {
    let header_text = concat!(
        "@HD\tVN:1.6\tSO:coordinate\n",
        "@SQ\tSN:chr1\tLN:100\tM5:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
        "@SQ\tSN:chr2\tLN:200\tUR:file://ref.fa\n",
        "@RG\tID:rg1\tSM:sample1\tPL:ILLUMINA\n",
        "@PG\tID:pg1\tPN:bamana\tVN:0.1.0\n",
        "@CO\toracle comparison fixture\n"
    );
    let path = write_bam_header_fixture(
        "header-oracle-valid",
        header_text,
        &[
            ReferenceFixture {
                name: "chr1",
                length: 100,
            },
            ReferenceFixture {
                name: "chr2",
                length: 200,
            },
        ],
    );

    let native = parse_bam_header(&path).expect("native header should parse");
    let oracle = read_noodles_header(&path);

    fs::remove_file(path).expect("fixture should be removed");
    assert_eq!(native.header.raw_header_text, header_text);
    assert!(native.header.reference_diagnostics.is_empty());
    assert_eq!(native.header.references.len(), oracle.references.len());
    for (native_reference, oracle_reference) in native
        .header
        .references
        .iter()
        .zip(oracle.references.iter())
    {
        assert_eq!(native_reference.name, oracle_reference.name);
        assert_eq!(native_reference.length, oracle_reference.length);
    }
    assert_eq!(native.header.read_groups.len(), oracle.read_group_count);
    assert_eq!(native.header.programs.len(), oracle.program_count);
    assert_eq!(native.header.comments.len(), oracle.comment_count);
}

#[test]
fn native_binary_authority_is_stricter_than_noodles_mismatch_policy() {
    let header_text = "@SQ\tSN:chr1\tLN:123\n";
    let path = write_bam_header_fixture(
        "header-oracle-mismatch",
        header_text,
        &[ReferenceFixture {
            name: "chr1",
            length: 456,
        }],
    );

    let native = parse_bam_header(&path).expect("native header should parse with diagnostics");
    let oracle_error = File::open(&path)
        .map(bam::io::Reader::new)
        .and_then(|mut reader| reader.read_header())
        .expect_err("noodles oracle should reject text/binary dictionary mismatch");

    fs::remove_file(path).expect("fixture should be removed");
    assert_eq!(native.header.references[0].length, 456);
    assert_eq!(native.header.references[0].text_header_length, Some(123));
    assert!(
        native
            .header
            .reference_diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic.kind == "reference_length_mismatch"
                    && diagnostic.binary_length == Some(456)
                    && diagnostic.text_length == Some(123)
            })
    );
    assert!(
        oracle_error.to_string().contains("dictionaries mismatch"),
        "unexpected oracle error: {oracle_error}"
    );
}

#[derive(Debug)]
struct ReferenceFixture {
    name: &'static str,
    length: u32,
}

#[derive(Debug)]
struct OracleHeader {
    references: Vec<OracleReference>,
    read_group_count: usize,
    program_count: usize,
    comment_count: usize,
}

#[derive(Debug)]
struct OracleReference {
    name: String,
    length: u32,
}

fn read_noodles_header(path: &Path) -> OracleHeader {
    let file = File::open(path).expect("oracle fixture should open");
    let mut reader = bam::io::Reader::new(file);
    let header = reader.read_header().expect("noodles oracle should parse");
    let references = header
        .reference_sequences()
        .iter()
        .map(|(name, reference)| OracleReference {
            name: String::from_utf8_lossy(name.as_ref()).into_owned(),
            length: usize::from(reference.length())
                .try_into()
                .expect("fixture reference length should fit u32"),
        })
        .collect();

    OracleHeader {
        references,
        read_group_count: header.read_groups().len(),
        program_count: header.programs().as_ref().len(),
        comment_count: header.comments().len(),
    }
}

fn write_bam_header_fixture(
    name: &str,
    header_text: &str,
    references: &[ReferenceFixture],
) -> PathBuf {
    let path = temp_path(name);
    let reference_records = references
        .iter()
        .enumerate()
        .map(|(index, reference)| bamana::bam::header::ReferenceRecord {
            name: reference.name.to_string(),
            length: reference.length,
            index,
            header_fields: bamana::bam::header::ReferenceHeaderFields::default(),
            text_header_length: None,
        })
        .collect::<Vec<_>>();
    let payload = serialize_bam_header_payload(&path, header_text, &reference_records)
        .expect("fixture header should serialize");
    let mut writer = BgzfWriter::create(&path).expect("fixture writer should create");
    writer
        .write_all(&payload)
        .expect("fixture payload should write");
    writer.finish().expect("fixture writer should finish");
    path
}

fn temp_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("bamana-{name}-{}-{nonce}.bam", std::process::id()))
}
