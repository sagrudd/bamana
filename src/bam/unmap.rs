use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    bam::{
        header::{
            parse_bam_header_from_reader, serialize_bam_header_payload, serialize_sam_header_text,
        },
        reader::BamReader,
        records::{BAM_FUNMAP, RecordLayout, read_next_record_layout},
        tags::{AuxField, traverse_aux_fields},
        write::{BgzfWriter, serialize_record_layout},
    },
    error::AppError,
};

const BAM_FPAIRED: u16 = 0x1;
const BAM_FPROPER_PAIR: u16 = 0x2;
const BAM_FMUNMAP: u16 = 0x8;
const BAM_FREVERSE: u16 = 0x10;
const BAM_FMREVERSE: u16 = 0x20;
const UNMAPPED_BIN: u16 = 4680;

const MAPPING_TAGS: [[u8; 2]; 31] = [
    *b"AM", *b"AS", *b"BQ", *b"CC", *b"CG", *b"CM", *b"CP", *b"H0", *b"H1", *b"H2", *b"HI", *b"IH",
    *b"MC", *b"MD", *b"MQ", *b"NH", *b"NM", *b"OA", *b"OC", *b"OP", *b"PQ", *b"SA", *b"SM", *b"UQ",
    *b"XS", *b"cm", *b"ms", *b"s1", *b"s2", *b"tp", *b"ts",
];

const ALIGNMENT_DIAGNOSTIC_TAGS: [[u8; 2]; 7] =
    [*b"cg", *b"cs", *b"de", *b"dv", *b"nn", *b"rl", *b"zd"];

#[derive(Debug, Clone)]
pub struct UnmapExecutionOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub dry_run: bool,
    pub threads: usize,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct UnmapExecution {
    pub overwritten: bool,
    pub records_read: u64,
    pub records_written: u64,
    pub tags_removed: u64,
    pub notes: Vec<String>,
}

pub fn unmap_bam(options: &UnmapExecutionOptions) -> Result<UnmapExecution, AppError> {
    if output_matches_input(&options.input_path, &options.output_path) {
        return Err(AppError::WriteError {
            path: options.output_path.clone(),
            message: "Output path must differ from the input BAM path.".to_string(),
        });
    }

    let preexisting_output = options.output_path.exists();
    if preexisting_output && !options.force {
        return Err(AppError::OutputExists {
            path: options.output_path.clone(),
        });
    }

    let mut reader = BamReader::open(&options.input_path)?;
    let parsed_header = parse_bam_header_from_reader(&mut reader)?;
    let mut rewritten_header = parsed_header.header.clone();
    rewritten_header.references.clear();
    rewritten_header.hd.sort_order = Some("unsorted".to_string());
    rewritten_header.hd.sub_sort_order = None;
    let rewritten_header_text = serialize_sam_header_text(&rewritten_header);

    let mut notes = vec![
        "Reference dictionary entries were removed from both the SAM header text and the BAM binary reference table."
            .to_string(),
        "Alignment coordinates, CIGAR data, mate coordinates, template length, and mapping-quality fields were stripped from every record."
            .to_string(),
        "Non-mapping auxiliary tags were preserved, including methylation-related tags such as MM and ML."
            .to_string(),
    ];
    if options.threads > 1 {
        notes.push(format!(
            "Thread count was set to {}, but this slice currently rewrites BAMs as a single streaming pass.",
            options.threads
        ));
    }

    let mut records_read = 0_u64;
    let mut records_written = 0_u64;
    let mut tags_removed = 0_u64;

    if options.dry_run {
        while let Some(layout) = read_next_record_layout(&mut reader)? {
            tags_removed += count_removed_mapping_tags(&layout.aux_bytes, &options.input_path)?;
            records_read += 1;
        }
        notes.push("Dry run only. No file modifications were made.".to_string());
        return Ok(UnmapExecution {
            overwritten: preexisting_output && options.force,
            records_read,
            records_written,
            tags_removed,
            notes,
        });
    }

    let header_payload = serialize_bam_header_payload(&rewritten_header_text, &[]);
    let temp_path = temporary_output_path(&options.output_path);
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    let write_result = (|| -> Result<(), AppError> {
        let mut writer = BgzfWriter::create(&temp_path)?;
        writer.write_all(&header_payload)?;

        while let Some(layout) = read_next_record_layout(&mut reader)? {
            let (unmapped, removed_here) = rewrite_record_as_unmapped(layout, &options.input_path)?;
            writer.write_all(&serialize_record_layout(&unmapped))?;
            records_read += 1;
            records_written += 1;
            tags_removed += removed_here;
        }

        writer.finish()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    if preexisting_output && options.force {
        fs::remove_file(&options.output_path).map_err(|error| AppError::WriteError {
            path: options.output_path.clone(),
            message: error.to_string(),
        })?;
    }
    fs::rename(&temp_path, &options.output_path).map_err(|error| AppError::WriteError {
        path: options.output_path.clone(),
        message: error.to_string(),
    })?;

    Ok(UnmapExecution {
        overwritten: preexisting_output && options.force,
        records_read,
        records_written,
        tags_removed,
        notes,
    })
}

fn rewrite_record_as_unmapped(
    mut record: RecordLayout,
    path: &Path,
) -> Result<(RecordLayout, u64), AppError> {
    let (filtered_aux, removed_tags) = remove_mapping_tags(&record.aux_bytes, path)?;
    let mut flags = record.flags | BAM_FUNMAP;
    flags &= !(BAM_FPROPER_PAIR | BAM_FREVERSE | BAM_FMREVERSE);
    if flags & BAM_FPAIRED != 0 {
        flags |= BAM_FMUNMAP;
    }

    record.ref_id = -1;
    record.pos = -1;
    record.bin = UNMAPPED_BIN;
    record.next_ref_id = -1;
    record.next_pos = -1;
    record.tlen = 0;
    record.flags = flags;
    record.mapping_quality = 0;
    record.n_cigar_op = 0;
    record.cigar_bytes.clear();
    record.aux_bytes = filtered_aux;
    record.block_size = 32
        + record.read_name.len()
        + 1
        + record.sequence_bytes.len()
        + record.quality_bytes.len()
        + record.aux_bytes.len();

    Ok((record, removed_tags))
}

fn remove_mapping_tags(aux_bytes: &[u8], path: &Path) -> Result<(Vec<u8>, u64), AppError> {
    let mut serialized = Vec::with_capacity(aux_bytes.len());
    let mut removed = 0_u64;

    traverse_aux_fields(aux_bytes, |field| {
        if is_mapping_tag(field.tag) {
            removed += 1;
        } else {
            serialize_aux_field(field, &mut serialized);
        }
        Ok(())
    })
    .map_err(|detail| AppError::TagParseUncertainty {
        path: path.to_path_buf(),
        detail,
    })?;

    Ok((serialized, removed))
}

fn count_removed_mapping_tags(aux_bytes: &[u8], path: &Path) -> Result<u64, AppError> {
    let mut removed = 0_u64;
    traverse_aux_fields(aux_bytes, |field| {
        if is_mapping_tag(field.tag) {
            removed += 1;
        }
        Ok(())
    })
    .map_err(|detail| AppError::TagParseUncertainty {
        path: path.to_path_buf(),
        detail,
    })?;
    Ok(removed)
}

fn serialize_aux_field(field: AuxField<'_>, target: &mut Vec<u8>) {
    target.extend_from_slice(&field.tag);
    target.push(field.type_code);
    target.extend_from_slice(field.payload);
}

fn is_mapping_tag(tag: [u8; 2]) -> bool {
    MAPPING_TAGS.contains(&tag) || ALIGNMENT_DIAGNOSTIC_TAGS.contains(&tag)
}

fn output_matches_input(input: &Path, output: &Path) -> bool {
    if input == output {
        return true;
    }

    let input_canonical = fs::canonicalize(input).ok();
    let output_canonical = fs::canonicalize(output).ok();
    input_canonical.is_some() && input_canonical == output_canonical
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let stem = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("bamana-unmap-output");
    output.with_file_name(format!(".{stem}.bamana-unmap-{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::{
        bam::{
            header::parse_bam_header,
            reader::BamReader,
            records::{
                BAM_FSECONDARY, BAM_FSUPPLEMENTARY, BAM_FUNMAP, RecordLayout,
                read_next_record_layout,
            },
            tags::{AuxTypeCode, TagQuery, aux_region_contains_tag},
            unmap::{UnmapExecutionOptions, unmap_bam},
            write::serialize_record_layout,
        },
        formats::bgzf::test_support::{build_bam_file_with_header_and_records, write_temp_file},
    };

    use super::rewrite_record_as_unmapped;

    #[test]
    fn unmap_rewrites_header_and_record_core_mapping_fields() {
        let input_record = RecordLayout {
            block_size: 0,
            ref_id: 0,
            pos: 99,
            bin: 123,
            next_ref_id: 0,
            next_pos: 199,
            tlen: 300,
            flags: 0x1 | 0x2 | 0x10 | 0x20,
            mapping_quality: 60,
            n_cigar_op: 1,
            l_seq: 4,
            read_name: "read1".to_string(),
            cigar_bytes: vec![0x40, 0x00, 0x00, 0x00],
            sequence_bytes: vec![0x12, 0x48],
            quality_bytes: vec![30, 30, 30, 30],
            aux_bytes: vec![
                b'N', b'M', b'C', 5, b'M', b'D', b'Z', b'4', 0, b'M', b'M', b'Z', b'C', b'+', b'm',
                b',', b'1', b';', 0, b'M', b'L', b'B', b'C', 1, 0, 0, 0, 42, b'R', b'G', b'Z',
                b'r', b'g', b'1', 0,
            ],
        };

        let input = write_temp_file(
            "unmap-input",
            "bam",
            &build_bam_file_with_header_and_records(
                "@HD\tVN:1.6\tSO:coordinate\tSS:coordinate:minhash\n@SQ\tSN:chr1\tLN:1000\n@RG\tID:rg1\tSM:sample\n",
                &[("chr1", 1000)],
                &[serialize_record_layout(&input_record)],
            ),
        );
        let output =
            std::env::temp_dir().join(format!("bamana-unmap-output-{}.bam", std::process::id()));

        let execution = unmap_bam(&UnmapExecutionOptions {
            input_path: input.clone(),
            output_path: output.clone(),
            dry_run: false,
            threads: 1,
            force: true,
        })
        .expect("unmap should succeed");

        let parsed_header = parse_bam_header(&output).expect("header should parse");
        assert!(parsed_header.header.references.is_empty());
        assert_eq!(
            parsed_header.header.hd.sort_order.as_deref(),
            Some("unsorted")
        );
        assert_eq!(parsed_header.header.hd.sub_sort_order, None);
        assert_eq!(parsed_header.header.read_groups.len(), 1);
        assert_eq!(execution.records_read, 1);
        assert_eq!(execution.records_written, 1);
        assert_eq!(execution.tags_removed, 2);

        let mut reader = BamReader::open(&output).expect("output should open");
        let _header = crate::bam::header::parse_bam_header_from_reader(&mut reader)
            .expect("output header should parse from reader");
        let record = read_next_record_layout(&mut reader)
            .expect("record should parse")
            .expect("record should exist");

        assert_eq!(record.ref_id, -1);
        assert_eq!(record.pos, -1);
        assert_eq!(record.next_ref_id, -1);
        assert_eq!(record.next_pos, -1);
        assert_eq!(record.tlen, 0);
        assert_eq!(record.mapping_quality, 0);
        assert_eq!(record.n_cigar_op, 0);
        assert!(record.cigar_bytes.is_empty());
        assert_eq!(record.flags & 0x4, 0x4);
        assert_eq!(record.flags & 0x8, 0x8);
        assert_eq!(record.flags & 0x2, 0);
        assert_eq!(record.flags & 0x10, 0);
        assert_eq!(record.flags & 0x20, 0);

        assert!(
            !aux_region_contains_tag(
                &record.aux_bytes,
                TagQuery {
                    tag: *b"NM",
                    required_type: Some(AuxTypeCode::CUpper),
                },
            )
            .expect("NM check should parse")
        );
        assert!(
            !aux_region_contains_tag(
                &record.aux_bytes,
                TagQuery {
                    tag: *b"MD",
                    required_type: Some(AuxTypeCode::Z),
                },
            )
            .expect("MD check should parse")
        );
        assert!(
            aux_region_contains_tag(
                &record.aux_bytes,
                TagQuery {
                    tag: *b"MM",
                    required_type: Some(AuxTypeCode::Z),
                },
            )
            .expect("MM check should parse")
        );
        assert!(
            aux_region_contains_tag(
                &record.aux_bytes,
                TagQuery {
                    tag: *b"ML",
                    required_type: Some(AuxTypeCode::B),
                },
            )
            .expect("ML check should parse")
        );
        assert!(
            aux_region_contains_tag(
                &record.aux_bytes,
                TagQuery {
                    tag: *b"RG",
                    required_type: Some(AuxTypeCode::Z),
                },
            )
            .expect("RG check should parse")
        );

        fs::remove_file(input).expect("fixture should be removable");
        fs::remove_file(output).expect("fixture should be removable");
    }

    #[test]
    fn rewrite_preserves_secondary_and_supplementary_flags() {
        let input = RecordLayout {
            block_size: 0,
            ref_id: 0,
            pos: 10,
            bin: 1,
            next_ref_id: -1,
            next_pos: -1,
            tlen: 0,
            flags: BAM_FSECONDARY | BAM_FSUPPLEMENTARY,
            mapping_quality: 42,
            n_cigar_op: 1,
            l_seq: 1,
            read_name: "r".to_string(),
            cigar_bytes: vec![0],
            sequence_bytes: vec![0x10],
            quality_bytes: vec![20],
            aux_bytes: Vec::new(),
        };

        let (output, removed_tags) =
            rewrite_record_as_unmapped(input, Path::new("fixture")).expect("rewrite should work");

        assert_eq!(removed_tags, 0);
        assert_eq!(output.flags & BAM_FSECONDARY, BAM_FSECONDARY);
        assert_eq!(output.flags & BAM_FSUPPLEMENTARY, BAM_FSUPPLEMENTARY);
        assert_eq!(output.flags & BAM_FUNMAP, BAM_FUNMAP);
    }
}
