use std::{fs::File, path::Path};

use crate::{
    error::BamanaError,
    formats::bgzf,
    json::schema::{BamHeader, HeaderHd, ProgramRecord, ReadGroup, ReferenceSequence},
};

const BAM_MAGIC: &[u8; 4] = b"BAM\x01";
const MAX_HEADER_BYTES: usize = 16 * 1024 * 1024;

pub fn bam_magic_present(path: &Path) -> Result<bool, BamanaError> {
    let prefix = bgzf::read_first_bgzf_payload(path)?;
    Ok(prefix.starts_with(BAM_MAGIC))
}

pub fn read_header(path: &Path) -> Result<BamHeader, BamanaError> {
    let mut file = File::open(path).map_err(|error| BamanaError::from_io(path, error))?;
    let mut buffer = Vec::new();

    loop {
        let Some(member) = bgzf::read_bgzf_member(&mut file, path)? else {
            return Err(BamanaError::TruncatedFile {
                path: path.to_path_buf(),
                detail: "Reached EOF before a complete BAM header was available.".to_string(),
            });
        };

        let payload = bgzf::decompress_member(&member, path)?;
        buffer.extend_from_slice(&payload);

        if buffer.len() > MAX_HEADER_BYTES {
            return Err(BamanaError::InvalidBam {
                path: path.to_path_buf(),
                detail: "BAM header exceeded the current 16 MiB safety limit.".to_string(),
            });
        }

        if let Some(header) = try_parse_header(&buffer, path)? {
            return Ok(header);
        }
    }
}

fn try_parse_header(bytes: &[u8], path: &Path) -> Result<Option<BamHeader>, BamanaError> {
    if bytes.len() < 8 {
        return Ok(None);
    }

    if &bytes[..4] != BAM_MAGIC {
        return Err(BamanaError::InvalidBam {
            path: path.to_path_buf(),
            detail: "Decompressed BGZF payload does not begin with BAM magic.".to_string(),
        });
    }

    let l_text = read_i32(bytes, 4, path)?;
    if l_text < 0 {
        return Err(BamanaError::InvalidBam {
            path: path.to_path_buf(),
            detail: "BAM header text length was negative.".to_string(),
        });
    }
    let l_text = l_text as usize;

    let text_end = 8 + l_text;
    if bytes.len() < text_end + 4 {
        return Ok(None);
    }

    let header_text = String::from_utf8(bytes[8..text_end].to_vec()).map_err(|error| {
        BamanaError::InvalidBam {
            path: path.to_path_buf(),
            detail: format!("BAM header text is not valid UTF-8: {error}"),
        }
    })?;

    let n_ref = read_i32(bytes, text_end, path)?;
    if n_ref < 0 {
        return Err(BamanaError::InvalidBam {
            path: path.to_path_buf(),
            detail: "BAM reference count was negative.".to_string(),
        });
    }

    let mut cursor = text_end + 4;
    let mut references = Vec::with_capacity(n_ref as usize);

    for _ in 0..n_ref {
        if bytes.len() < cursor + 4 {
            return Ok(None);
        }

        let l_name = read_i32(bytes, cursor, path)?;
        if l_name <= 0 {
            return Err(BamanaError::InvalidBam {
                path: path.to_path_buf(),
                detail: "BAM reference name length was not positive.".to_string(),
            });
        }
        let l_name = l_name as usize;
        cursor += 4;

        if bytes.len() < cursor + l_name + 4 {
            return Ok(None);
        }

        let name_bytes = &bytes[cursor..cursor + l_name];
        if name_bytes.last().copied() != Some(0) {
            return Err(BamanaError::InvalidBam {
                path: path.to_path_buf(),
                detail: "BAM reference name was not NUL-terminated.".to_string(),
            });
        }

        let name = String::from_utf8(name_bytes[..l_name - 1].to_vec()).map_err(|error| {
            BamanaError::InvalidBam {
                path: path.to_path_buf(),
                detail: format!("BAM reference name is not valid UTF-8: {error}"),
            }
        })?;
        cursor += l_name;

        let length = read_i32(bytes, cursor, path)?;
        if length < 0 {
            return Err(BamanaError::InvalidBam {
                path: path.to_path_buf(),
                detail: "BAM reference length was negative.".to_string(),
            });
        }
        cursor += 4;

        references.push(ReferenceSequence {
            name,
            length: length as u32,
        });
    }

    let (hd, read_groups, programs, comments) = parse_header_text(&header_text);
    Ok(Some(BamHeader {
        hd,
        references,
        read_groups,
        programs,
        comments,
    }))
}

fn parse_header_text(
    header_text: &str,
) -> (HeaderHd, Vec<ReadGroup>, Vec<ProgramRecord>, Vec<String>) {
    let mut hd = HeaderHd::default();
    let mut read_groups = Vec::new();
    let mut programs = Vec::new();
    let mut comments = Vec::new();

    for line in header_text.lines() {
        let mut fields = line.split('\t');
        let Some(record_type) = fields.next() else {
            continue;
        };

        match record_type {
            "@HD" => {
                for field in fields {
                    if let Some((tag, value)) = field.split_once(':') {
                        match tag {
                            "VN" => hd.version = Some(value.to_string()),
                            "SO" => hd.sort_order = Some(value.to_string()),
                            "SS" => hd.sub_sort_order = Some(value.to_string()),
                            _ => {}
                        }
                    }
                }
            }
            "@RG" => {
                let mut rg = ReadGroup::default();
                for field in fields {
                    if let Some((tag, value)) = field.split_once(':') {
                        match tag {
                            "ID" => rg.id = Some(value.to_string()),
                            "SM" => rg.sample = Some(value.to_string()),
                            "LB" => rg.library = Some(value.to_string()),
                            "PL" => rg.platform = Some(value.to_string()),
                            _ => {}
                        }
                    }
                }
                read_groups.push(rg);
            }
            "@PG" => {
                let mut program = ProgramRecord::default();
                for field in fields {
                    if let Some((tag, value)) = field.split_once(':') {
                        match tag {
                            "ID" => program.id = Some(value.to_string()),
                            "PN" => program.name = Some(value.to_string()),
                            "VN" => program.version = Some(value.to_string()),
                            "CL" => program.command_line = Some(value.to_string()),
                            _ => {}
                        }
                    }
                }
                programs.push(program);
            }
            "@CO" => {
                comments.push(fields.collect::<Vec<_>>().join("\t"));
            }
            _ => {}
        }
    }

    // TODO: preserve and expose additional header records and tags once the
    // public JSON schema for broader BAM header reporting is finalized.
    (hd, read_groups, programs, comments)
}

fn read_i32(bytes: &[u8], offset: usize, path: &Path) -> Result<i32, BamanaError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| BamanaError::TruncatedFile {
            path: path.to_path_buf(),
            detail: "BAM header ended unexpectedly while reading a 32-bit field.".to_string(),
        })?;

    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::formats::bgzf::test_support::build_bam_file;

    use super::{bam_magic_present, read_header};

    fn write_test_file(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "bamana-{name}-{}-{}.bam",
            std::process::id(),
            std::thread::current().name().unwrap_or("main")
        ));
        fs::write(&path, bytes).expect("test fixture should be written");
        path
    }

    #[test]
    fn detects_bam_magic() {
        let bam = build_bam_file("@HD\tVN:1.6\tSO:coordinate\n", &[("chr1", 248_956_422)]);
        let path = write_test_file("magic", &bam);
        let result = bam_magic_present(&path).expect("bam magic should be readable");
        fs::remove_file(path).expect("test fixture should be removable");
        assert!(result);
    }

    #[test]
    fn parses_header_fields() {
        let bam = build_bam_file(
            "@HD\tVN:1.6\tSO:coordinate\n@RG\tID:rg1\tSM:sample1\n@PG\tID:prog1\tPN:bamana\tVN:0.1.0\n@CO\tgenerated for tests\n",
            &[("chr1", 248_956_422)],
        );
        let path = write_test_file("header", &bam);
        let header = read_header(&path).expect("header should parse");
        fs::remove_file(path).expect("test fixture should be removable");

        assert_eq!(header.hd.version.as_deref(), Some("1.6"));
        assert_eq!(header.hd.sort_order.as_deref(), Some("coordinate"));
        assert_eq!(header.references.len(), 1);
        assert_eq!(header.references[0].name, "chr1");
        assert_eq!(header.read_groups[0].id.as_deref(), Some("rg1"));
        assert_eq!(header.programs[0].name.as_deref(), Some("bamana"));
        assert_eq!(header.comments[0], "generated for tests");
    }
}
