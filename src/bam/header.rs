use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::{bam::reader::BamReader, error::AppError};

const BAM_MAGIC: &[u8; 4] = b"BAM\x01";
const MAX_HEADER_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_REFERENCE_COUNT: usize = 1_000_000;
const MAX_REFERENCE_NAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct HeaderPayload {
    pub format: &'static str,
    pub header: BamHeaderView,
}

#[derive(Debug, Serialize)]
pub struct BamHeaderView {
    pub raw_header_text: String,
    pub hd: HdRecord,
    pub references: Vec<ReferenceRecord>,
    pub read_groups: Vec<ReadGroupRecord>,
    pub programs: Vec<ProgramRecord>,
    pub comments: Vec<String>,
    pub other_header_records: Vec<OtherHeaderRecord>,
}

#[derive(Debug, Default, Serialize)]
pub struct HdRecord {
    pub version: Option<String>,
    pub sort_order: Option<String>,
    pub sub_sort_order: Option<String>,
    pub group_order: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct ReferenceHeaderFields {
    #[serde(rename = "M5")]
    pub m5: Option<String>,
    #[serde(rename = "UR")]
    pub ur: Option<String>,
    #[serde(rename = "AS")]
    pub assembly: Option<String>,
    #[serde(rename = "SP")]
    pub species: Option<String>,
    #[serde(rename = "TP")]
    pub topology: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReferenceRecord {
    pub name: String,
    pub length: u32,
    pub index: usize,
    pub header_fields: ReferenceHeaderFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_header_length: Option<u32>,
}

#[derive(Debug, Default, Serialize)]
pub struct ReadGroupRecord {
    #[serde(rename = "ID")]
    pub id: Option<String>,
    #[serde(rename = "SM")]
    pub sample: Option<String>,
    #[serde(rename = "LB")]
    pub library: Option<String>,
    #[serde(rename = "PL")]
    pub platform: Option<String>,
    #[serde(rename = "PU")]
    pub platform_unit: Option<String>,
    #[serde(rename = "CN")]
    pub center: Option<String>,
    #[serde(rename = "DS")]
    pub description: Option<String>,
    #[serde(rename = "DT")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub other_fields: BTreeMap<String, String>,
}

#[derive(Debug, Default, Serialize)]
pub struct ProgramRecord {
    #[serde(rename = "ID")]
    pub id: Option<String>,
    #[serde(rename = "PN")]
    pub name: Option<String>,
    #[serde(rename = "VN")]
    pub version: Option<String>,
    #[serde(rename = "CL")]
    pub command_line: Option<String>,
    #[serde(rename = "PP")]
    pub previous_program_id: Option<String>,
    #[serde(rename = "DS")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub other_fields: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct OtherHeaderRecord {
    pub record_type: String,
    pub raw_line: String,
}

#[derive(Debug)]
struct BinaryReference {
    name: String,
    length: u32,
}

#[derive(Debug, Default)]
struct ParsedSamHeader {
    hd: HdRecord,
    sq: HashMap<String, SamSqRecord>,
    read_groups: Vec<ReadGroupRecord>,
    programs: Vec<ProgramRecord>,
    comments: Vec<String>,
    other_header_records: Vec<OtherHeaderRecord>,
}

#[derive(Debug, Default)]
struct SamSqRecord {
    length: Option<u32>,
    fields: ReferenceHeaderFields,
}

pub fn parse_bam_header(path: &std::path::Path) -> Result<HeaderPayload, AppError> {
    let mut reader = BamReader::open(path)?;
    parse_bam_header_from_reader(&mut reader)
}

pub fn parse_bam_header_from_reader(reader: &mut BamReader) -> Result<HeaderPayload, AppError> {
    let magic = reader.read_magic()?;
    if &magic != BAM_MAGIC {
        return Err(AppError::InvalidHeader {
            path: reader.path().to_path_buf(),
            detail: "Missing BAM magic in decompressed stream.".to_string(),
        });
    }

    let l_text = reader.read_i32_le()?;
    if l_text < 0 {
        return Err(AppError::InvalidHeader {
            path: reader.path().to_path_buf(),
            detail: "BAM header text length was negative.".to_string(),
        });
    }
    let l_text = l_text as usize;
    if l_text > MAX_HEADER_TEXT_BYTES {
        return Err(AppError::InvalidHeader {
            path: reader.path().to_path_buf(),
            detail: format!(
                "BAM header text length {l_text} exceeds the current safety limit of {MAX_HEADER_TEXT_BYTES} bytes."
            ),
        });
    }

    let raw_header_text = String::from_utf8(reader.read_exact_vec(l_text)?).map_err(|error| {
        AppError::InvalidHeader {
            path: reader.path().to_path_buf(),
            detail: format!("BAM header text is not valid UTF-8: {error}"),
        }
    })?;

    let n_ref = reader.read_i32_le()?;
    if n_ref < 0 {
        return Err(AppError::InvalidHeader {
            path: reader.path().to_path_buf(),
            detail: "BAM reference count was negative.".to_string(),
        });
    }
    let n_ref = n_ref as usize;
    if n_ref > MAX_REFERENCE_COUNT {
        return Err(AppError::InvalidHeader {
            path: reader.path().to_path_buf(),
            detail: format!(
                "BAM reference count {n_ref} exceeds the current safety limit of {MAX_REFERENCE_COUNT}."
            ),
        });
    }

    let mut binary_references = Vec::with_capacity(n_ref);
    for _ in 0..n_ref {
        let l_name = reader.read_i32_le()?;
        if l_name <= 0 {
            return Err(AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: "BAM reference name length was not positive.".to_string(),
            });
        }
        let l_name = l_name as usize;
        if l_name > MAX_REFERENCE_NAME_BYTES {
            return Err(AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: format!(
                    "BAM reference name length {l_name} exceeds the current safety limit of {MAX_REFERENCE_NAME_BYTES} bytes."
                ),
            });
        }

        let name_bytes = reader.read_exact_vec(l_name)?;
        let Some((&0, name_without_nul)) = name_bytes.split_last() else {
            return Err(AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: "BAM reference name was not NUL-terminated.".to_string(),
            });
        };

        let name = String::from_utf8(name_without_nul.to_vec()).map_err(|error| {
            AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: format!("BAM reference name is not valid UTF-8: {error}"),
            }
        })?;

        let l_ref = reader.read_i32_le()?;
        if l_ref < 0 {
            return Err(AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: "BAM reference length was negative.".to_string(),
            });
        }

        binary_references.push(BinaryReference {
            name,
            length: l_ref as u32,
        });
    }

    let sam_header = parse_sam_header_text(&raw_header_text);
    let references = merge_references(binary_references, &sam_header.sq);

    Ok(HeaderPayload {
        format: "BAM",
        header: BamHeaderView {
            raw_header_text,
            hd: sam_header.hd,
            references,
            read_groups: sam_header.read_groups,
            programs: sam_header.programs,
            comments: sam_header.comments,
            other_header_records: sam_header.other_header_records,
        },
    })
}

pub fn rewrite_header_for_sort(
    raw_header_text: &str,
    sort_order: &str,
    sub_sort_order: Option<&str>,
) -> String {
    let mut lines = Vec::new();
    let mut updated_hd = false;

    for line in raw_header_text.lines() {
        if line.starts_with("@HD") && !updated_hd {
            lines.push(rewrite_hd_line(line, sort_order, sub_sort_order));
            updated_hd = true;
        } else if !line.is_empty() {
            lines.push(line.to_string());
        }
    }

    if !updated_hd {
        lines.insert(0, build_hd_line(Vec::new(), sort_order, sub_sort_order));
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

pub fn serialize_bam_header_payload(header_text: &str, references: &[ReferenceRecord]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(BAM_MAGIC);
    payload.extend_from_slice(&(header_text.len() as i32).to_le_bytes());
    payload.extend_from_slice(header_text.as_bytes());
    payload.extend_from_slice(&(references.len() as i32).to_le_bytes());

    for reference in references {
        let mut name = reference.name.as_bytes().to_vec();
        name.push(0);
        payload.extend_from_slice(&(name.len() as i32).to_le_bytes());
        payload.extend_from_slice(&name);
        payload.extend_from_slice(&(reference.length as i32).to_le_bytes());
    }

    payload
}

fn parse_sam_header_text(raw_header_text: &str) -> ParsedSamHeader {
    let mut parsed = ParsedSamHeader::default();

    for line in raw_header_text.lines() {
        if line.is_empty() {
            continue;
        }

        let mut fields = line.split('\t');
        let Some(record_type) = fields.next() else {
            continue;
        };

        match record_type {
            "@HD" => {
                for (tag, value) in parse_tag_fields(fields) {
                    match tag.as_str() {
                        "VN" => parsed.hd.version = Some(value),
                        "SO" => parsed.hd.sort_order = Some(value),
                        "SS" => parsed.hd.sub_sort_order = Some(value),
                        "GO" => parsed.hd.group_order = Some(value),
                        _ => {}
                    }
                }
            }
            "@SQ" => {
                let mut name = None;
                let mut sq = SamSqRecord::default();
                for (tag, value) in parse_tag_fields(fields) {
                    match tag.as_str() {
                        "SN" => name = Some(value),
                        "LN" => sq.length = value.parse::<u32>().ok(),
                        "M5" => sq.fields.m5 = Some(value),
                        "UR" => sq.fields.ur = Some(value),
                        "AS" => sq.fields.assembly = Some(value),
                        "SP" => sq.fields.species = Some(value),
                        "TP" => sq.fields.topology = Some(value),
                        _ => {}
                    }
                }
                if let Some(name) = name {
                    parsed.sq.insert(name, sq);
                }
            }
            "@RG" => {
                let mut rg = ReadGroupRecord::default();
                for (tag, value) in parse_tag_fields(fields) {
                    match tag.as_str() {
                        "ID" => rg.id = Some(value),
                        "SM" => rg.sample = Some(value),
                        "LB" => rg.library = Some(value),
                        "PL" => rg.platform = Some(value),
                        "PU" => rg.platform_unit = Some(value),
                        "CN" => rg.center = Some(value),
                        "DS" => rg.description = Some(value),
                        "DT" => rg.date = Some(value),
                        _ => {
                            rg.other_fields.insert(tag, value);
                        }
                    }
                }
                parsed.read_groups.push(rg);
            }
            "@PG" => {
                let mut pg = ProgramRecord::default();
                for (tag, value) in parse_tag_fields(fields) {
                    match tag.as_str() {
                        "ID" => pg.id = Some(value),
                        "PN" => pg.name = Some(value),
                        "VN" => pg.version = Some(value),
                        "CL" => pg.command_line = Some(value),
                        "PP" => pg.previous_program_id = Some(value),
                        "DS" => pg.description = Some(value),
                        _ => {
                            pg.other_fields.insert(tag, value);
                        }
                    }
                }
                parsed.programs.push(pg);
            }
            "@CO" => {
                parsed.comments.push(fields.collect::<Vec<_>>().join("\t"));
            }
            _ if record_type.starts_with('@') => {
                parsed.other_header_records.push(OtherHeaderRecord {
                    record_type: record_type.trim_start_matches('@').to_string(),
                    raw_line: line.to_string(),
                });
            }
            _ => {}
        }
    }

    parsed
}

fn rewrite_hd_line(line: &str, sort_order: &str, sub_sort_order: Option<&str>) -> String {
    let mut preserved_fields = Vec::new();

    for field in line.split('\t').skip(1) {
        let Some((key, value)) = field.split_once(':') else {
            continue;
        };
        if matches!(key, "SO" | "SS" | "GO") {
            continue;
        }
        preserved_fields.push((key.to_string(), value.to_string()));
    }

    build_hd_line(preserved_fields, sort_order, sub_sort_order)
}

fn build_hd_line(
    mut preserved_fields: Vec<(String, String)>,
    sort_order: &str,
    sub_sort_order: Option<&str>,
) -> String {
    let mut line = String::from("@HD");
    preserved_fields.retain(|(key, _)| !matches!(key.as_str(), "SO" | "SS" | "GO"));
    for (key, value) in preserved_fields {
        line.push('\t');
        line.push_str(&key);
        line.push(':');
        line.push_str(&value);
    }
    line.push_str("\tSO:");
    line.push_str(sort_order);
    if let Some(sub_sort_order) = sub_sort_order {
        line.push_str("\tSS:");
        line.push_str(sub_sort_order);
    }
    line
}

fn parse_tag_fields<'a>(fields: impl Iterator<Item = &'a str>) -> Vec<(String, String)> {
    fields
        .filter_map(|field| {
            let (tag, value) = field.split_once(':')?;
            Some((tag.to_string(), value.to_string()))
        })
        .collect()
}

fn merge_references(
    binary_references: Vec<BinaryReference>,
    sq_map: &HashMap<String, SamSqRecord>,
) -> Vec<ReferenceRecord> {
    binary_references
        .into_iter()
        .enumerate()
        .map(|(index, binary_reference)| {
            let sq = sq_map.get(&binary_reference.name);
            ReferenceRecord {
                name: binary_reference.name,
                length: binary_reference.length,
                index,
                header_fields: sq
                    .map(|sq| ReferenceHeaderFields {
                        m5: sq.fields.m5.clone(),
                        ur: sq.fields.ur.clone(),
                        assembly: sq.fields.assembly.clone(),
                        species: sq.fields.species.clone(),
                        topology: sq.fields.topology.clone(),
                    })
                    .unwrap_or_default(),
                text_header_length: sq
                    .and_then(|sq| sq.length)
                    .filter(|length| *length != binary_reference.length),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::parse_bam_header;
    use crate::formats::bgzf::test_support::{build_bam_file_with_header, write_temp_file};

    #[test]
    fn parses_binary_and_text_header_sections() {
        let header_text = concat!(
            "@HD\tVN:1.6\tSO:coordinate\tGO:query\n",
            "@SQ\tSN:chr1\tLN:248956422\tM5:abc123\tUR:file://ref.fa\n",
            "@SQ\tSN:chr2\tLN:242193529\tAS:GRCh38\tSP:Homo sapiens\tTP:linear\n",
            "@RG\tID:rg1\tSM:sample1\tPL:ILLUMINA\tPU:unit1\n",
            "@PG\tID:pg1\tPN:bamana\tVN:0.1.0\tCL:bamana header --bam test.bam\n",
            "@CO\tgenerated for tests\n",
            "@XY\tZZ:custom\n"
        );
        let bytes = build_bam_file_with_header(
            header_text,
            &[("chr1", 248_956_422), ("chr2", 242_193_529)],
        );
        let path = write_temp_file("header-parse", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(payload.format, "BAM");
        assert_eq!(payload.header.hd.version.as_deref(), Some("1.6"));
        assert_eq!(payload.header.hd.sort_order.as_deref(), Some("coordinate"));
        assert_eq!(payload.header.hd.group_order.as_deref(), Some("query"));
        assert_eq!(payload.header.references.len(), 2);
        assert_eq!(payload.header.references[0].name, "chr1");
        assert_eq!(
            payload.header.references[0].header_fields.m5.as_deref(),
            Some("abc123")
        );
        assert_eq!(
            payload.header.references[1]
                .header_fields
                .assembly
                .as_deref(),
            Some("GRCh38")
        );
        assert_eq!(payload.header.read_groups[0].id.as_deref(), Some("rg1"));
        assert_eq!(payload.header.programs[0].id.as_deref(), Some("pg1"));
        assert_eq!(payload.header.comments[0], "generated for tests");
        assert_eq!(payload.header.other_header_records[0].record_type, "XY");
    }

    #[test]
    fn preserves_binary_reference_length_when_text_sq_disagrees() {
        let header_text = "@SQ\tSN:chr1\tLN:123\n";
        let bytes = build_bam_file_with_header(header_text, &[("chr1", 456)]);
        let path = write_temp_file("header-mismatch", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(payload.header.references[0].length, 456);
        assert_eq!(payload.header.references[0].text_header_length, Some(123));
    }
}
