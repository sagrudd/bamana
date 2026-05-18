use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use serde::Serialize;

use crate::{bam::reader::BamReader, error::AppError};

const BAM_MAGIC: &[u8; 4] = b"BAM\x01";
const MAX_HEADER_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_REFERENCE_COUNT: usize = 1_000_000;
const MAX_REFERENCE_NAME_BYTES: usize = 1024 * 1024;
const MAX_BAM_I32_FIELD: u32 = i32::MAX as u32;

/// JSON-facing BAM header payload produced by the native header codec.
#[derive(Debug, Clone, Serialize)]
pub struct HeaderPayload {
    pub format: &'static str,
    pub header: BamHeaderView,
}

/// Stable native representation shared by header, writer, reheader, merge,
/// checksum, and validation consumers.
///
/// `raw_header_text` preserves the declared SAM-style text exactly as parsed
/// from BAM, while `references` is the binary reference dictionary in encounter
/// order. Binary reference names, lengths, and indexes are authoritative for
/// BAM decoding; parsed textual fields are retained as diagnostics and
/// user-facing metadata.
#[derive(Debug, Clone, Serialize)]
pub struct BamHeaderView {
    pub raw_header_text: String,
    pub hd: HdRecord,
    pub references: Vec<ReferenceRecord>,
    pub reference_diagnostics: Vec<ReferenceDiagnostic>,
    pub read_groups: Vec<ReadGroupRecord>,
    pub programs: Vec<ProgramRecord>,
    pub comments: Vec<String>,
    pub other_header_records: Vec<OtherHeaderRecord>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HdRecord {
    pub version: Option<String>,
    pub sort_order: Option<String>,
    pub sub_sort_order: Option<String>,
    pub group_order: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceRecord {
    /// Binary reference name without the required BAM NUL terminator.
    pub name: String,
    /// Binary reference length. Values must fit the non-negative BAM `i32`
    /// field before serialization.
    pub length: u32,
    /// Encounter-order index from the binary reference dictionary.
    pub index: usize,
    /// Metadata copied from the matching textual `@SQ` record, if present.
    pub header_fields: ReferenceHeaderFields,
    /// Textual `@SQ` `LN` when it disagrees with the binary length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_header_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceDiagnostic {
    pub kind: &'static str,
    pub severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_length: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
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

#[derive(Debug, Clone, Default, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct OtherHeaderRecord {
    pub record_type: String,
    pub raw_line: String,
}

#[derive(Debug, Clone)]
struct BinaryReference {
    name: String,
    length: u32,
}

#[derive(Debug, Default)]
struct ParsedSamHeader {
    hd: HdRecord,
    sq: HashMap<String, SamSqRecord>,
    sq_records: Vec<TextSqRecord>,
    sq_order: Vec<String>,
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

#[derive(Debug, Clone)]
struct TextSqRecord {
    index: usize,
    name: Option<String>,
    length: Option<u32>,
    raw_line: String,
}

pub fn parse_bam_header(path: &Path) -> Result<HeaderPayload, AppError> {
    let mut reader = BamReader::open(path)?;
    parse_bam_header_from_reader(&mut reader)
}

pub fn parse_bam_header_from_native_bgzf(path: &Path) -> Result<HeaderPayload, AppError> {
    let mut reader = BamReader::open_native_bgzf(path)?;
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

    let l_text = reader.read_i32_le_with_context("BAM stream ended while reading l_text.")?;
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

    let raw_header_text = String::from_utf8(
        reader
            .read_exact_vec_with_context(l_text, "BAM stream ended while reading header text.")?,
    )
    .map_err(|error| AppError::InvalidHeader {
        path: reader.path().to_path_buf(),
        detail: format!("BAM header text is not valid UTF-8: {error}"),
    })?;

    let n_ref =
        reader.read_i32_le_with_context("BAM stream ended while reading reference count.")?;
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
        let l_name = reader
            .read_i32_le_with_context("BAM stream ended while reading reference name length.")?;
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

        let name_bytes = reader.read_exact_vec_with_context(
            l_name,
            "BAM stream ended while reading reference name.",
        )?;
        let Some((&0, name_without_nul)) = name_bytes.split_last() else {
            return Err(AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: "BAM reference name was not NUL-terminated.".to_string(),
            });
        };
        if name_without_nul.is_empty() {
            return Err(AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: "BAM reference name was empty.".to_string(),
            });
        }
        if name_without_nul.contains(&0) {
            return Err(AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: "BAM reference name contained an interior NUL byte.".to_string(),
            });
        }

        let name = String::from_utf8(name_without_nul.to_vec()).map_err(|error| {
            AppError::InvalidHeader {
                path: reader.path().to_path_buf(),
                detail: format!("BAM reference name is not valid UTF-8: {error}"),
            }
        })?;

        let l_ref =
            reader.read_i32_le_with_context("BAM stream ended while reading reference length.")?;
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

    let sam_header = parse_sam_header_text_impl(&raw_header_text);
    let reference_diagnostics = reconcile_reference_diagnostics(&binary_references, &sam_header);
    let references = merge_references(binary_references, &sam_header.sq);

    Ok(HeaderPayload {
        format: "BAM",
        header: BamHeaderView {
            raw_header_text,
            hd: sam_header.hd,
            references,
            reference_diagnostics,
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

pub fn serialize_bam_header_payload(
    path: &Path,
    header_text: &str,
    references: &[ReferenceRecord],
) -> Result<Vec<u8>, AppError> {
    let mut payload = Vec::new();
    payload.extend_from_slice(BAM_MAGIC);
    payload.extend_from_slice(
        &checked_bam_i32_from_usize(path, "BAM header text length", header_text.len())?
            .to_le_bytes(),
    );
    payload.extend_from_slice(header_text.as_bytes());
    payload.extend_from_slice(
        &checked_bam_i32_from_usize(path, "BAM reference count", references.len())?.to_le_bytes(),
    );

    for reference in references {
        let mut name = reference.name.as_bytes().to_vec();
        name.push(0);
        payload.extend_from_slice(
            &checked_bam_i32_from_usize(path, "BAM reference name length", name.len())?
                .to_le_bytes(),
        );
        payload.extend_from_slice(&name);
        payload.extend_from_slice(
            &checked_bam_i32_from_u32(path, "BAM reference length", reference.length)?
                .to_le_bytes(),
        );
    }

    Ok(payload)
}

pub fn serialize_bam_header_view_payload(
    path: &Path,
    header: &BamHeaderView,
) -> Result<Vec<u8>, AppError> {
    serialize_bam_header_payload(path, &header.raw_header_text, &header.references)
}

pub fn serialize_bam_header_checksum_domain(header: &HeaderPayload) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_len_prefixed(&mut bytes, header.header.raw_header_text.as_bytes());
    bytes.extend_from_slice(&(header.header.references.len() as u32).to_le_bytes());
    for reference in &header.header.references {
        write_len_prefixed(&mut bytes, reference.name.as_bytes());
        bytes.extend_from_slice(&reference.length.to_le_bytes());
    }
    bytes
}

pub fn parse_sam_header_text_with_references(
    raw_header_text: &str,
    references: &[ReferenceRecord],
) -> Result<BamHeaderView, String> {
    let normalized = normalize_header_text(raw_header_text);
    for line in normalized.lines() {
        if !line.is_empty() && !line.starts_with('@') {
            return Err(
                "Replacement header file contained non-header SAM content; only header lines are allowed."
                    .to_string(),
            );
        }
    }
    let parsed = parse_sam_header_text_impl(&normalized);
    validate_reheader_reference_dictionary(&parsed, references)?;
    validate_unique_read_groups(&parsed.read_groups)?;
    validate_unique_programs(&parsed.programs)?;

    let binary_references: Vec<BinaryReference> = references
        .iter()
        .map(|reference| BinaryReference {
            name: reference.name.clone(),
            length: reference.length,
        })
        .collect();
    let reference_diagnostics = reconcile_reference_diagnostics(&binary_references, &parsed);
    let merged_references = merge_references(binary_references, &parsed.sq);

    Ok(BamHeaderView {
        raw_header_text: normalized,
        hd: parsed.hd,
        references: merged_references,
        reference_diagnostics,
        read_groups: parsed.read_groups,
        programs: parsed.programs,
        comments: parsed.comments,
        other_header_records: parsed.other_header_records,
    })
}

pub fn serialize_sam_header_text(header: &BamHeaderView) -> String {
    let mut lines = Vec::new();

    if header.hd.version.is_some()
        || header.hd.sort_order.is_some()
        || header.hd.sub_sort_order.is_some()
        || header.hd.group_order.is_some()
    {
        let mut line = String::from("@HD");
        if let Some(version) = &header.hd.version {
            line.push_str("\tVN:");
            line.push_str(version);
        }
        if let Some(sort_order) = &header.hd.sort_order {
            line.push_str("\tSO:");
            line.push_str(sort_order);
        }
        if let Some(sub_sort_order) = &header.hd.sub_sort_order {
            line.push_str("\tSS:");
            line.push_str(sub_sort_order);
        }
        if let Some(group_order) = &header.hd.group_order {
            line.push_str("\tGO:");
            line.push_str(group_order);
        }
        lines.push(line);
    }

    for reference in &header.references {
        let mut line = format!("@SQ\tSN:{}\tLN:{}", reference.name, reference.length);
        if let Some(m5) = &reference.header_fields.m5 {
            line.push_str("\tM5:");
            line.push_str(m5);
        }
        if let Some(ur) = &reference.header_fields.ur {
            line.push_str("\tUR:");
            line.push_str(ur);
        }
        if let Some(assembly) = &reference.header_fields.assembly {
            line.push_str("\tAS:");
            line.push_str(assembly);
        }
        if let Some(species) = &reference.header_fields.species {
            line.push_str("\tSP:");
            line.push_str(species);
        }
        if let Some(topology) = &reference.header_fields.topology {
            line.push_str("\tTP:");
            line.push_str(topology);
        }
        lines.push(line);
    }

    for rg in &header.read_groups {
        let mut line = String::from("@RG");
        if let Some(id) = &rg.id {
            line.push_str("\tID:");
            line.push_str(id);
        }
        if let Some(sample) = &rg.sample {
            line.push_str("\tSM:");
            line.push_str(sample);
        }
        if let Some(library) = &rg.library {
            line.push_str("\tLB:");
            line.push_str(library);
        }
        if let Some(platform) = &rg.platform {
            line.push_str("\tPL:");
            line.push_str(platform);
        }
        if let Some(platform_unit) = &rg.platform_unit {
            line.push_str("\tPU:");
            line.push_str(platform_unit);
        }
        if let Some(center) = &rg.center {
            line.push_str("\tCN:");
            line.push_str(center);
        }
        if let Some(description) = &rg.description {
            line.push_str("\tDS:");
            line.push_str(description);
        }
        if let Some(date) = &rg.date {
            line.push_str("\tDT:");
            line.push_str(date);
        }
        for (tag, value) in &rg.other_fields {
            line.push('\t');
            line.push_str(tag);
            line.push(':');
            line.push_str(value);
        }
        lines.push(line);
    }

    for pg in &header.programs {
        let mut line = String::from("@PG");
        if let Some(id) = &pg.id {
            line.push_str("\tID:");
            line.push_str(id);
        }
        if let Some(name) = &pg.name {
            line.push_str("\tPN:");
            line.push_str(name);
        }
        if let Some(version) = &pg.version {
            line.push_str("\tVN:");
            line.push_str(version);
        }
        if let Some(command_line) = &pg.command_line {
            line.push_str("\tCL:");
            line.push_str(command_line);
        }
        if let Some(previous_program_id) = &pg.previous_program_id {
            line.push_str("\tPP:");
            line.push_str(previous_program_id);
        }
        if let Some(description) = &pg.description {
            line.push_str("\tDS:");
            line.push_str(description);
        }
        for (tag, value) in &pg.other_fields {
            line.push('\t');
            line.push_str(tag);
            line.push(':');
            line.push_str(value);
        }
        lines.push(line);
    }

    for comment in &header.comments {
        lines.push(format!("@CO\t{comment}"));
    }

    for record in &header.other_header_records {
        if record.raw_line.starts_with('@') {
            lines.push(record.raw_line.clone());
        } else {
            lines.push(format!("@{}\t{}", record.record_type, record.raw_line));
        }
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn parse_sam_header_text_impl(raw_header_text: &str) -> ParsedSamHeader {
    let mut parsed = ParsedSamHeader::default();

    for line in raw_header_text.lines() {
        if line.is_empty() {
            continue;
        }

        if !line.starts_with('@') {
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
                let record = TextSqRecord {
                    index: parsed.sq_records.len(),
                    name: name.clone(),
                    length: sq.length,
                    raw_line: line.to_string(),
                };
                parsed.sq_records.push(record);
                if let Some(name) = name {
                    parsed.sq_order.push(name.clone());
                    parsed.sq.entry(name).or_insert(sq);
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

fn normalize_header_text(raw_header_text: &str) -> String {
    if raw_header_text.is_empty() {
        String::new()
    } else if raw_header_text.ends_with('\n') {
        raw_header_text.to_string()
    } else {
        format!("{raw_header_text}\n")
    }
}

fn validate_reheader_reference_dictionary(
    parsed: &ParsedSamHeader,
    references: &[ReferenceRecord],
) -> Result<(), String> {
    if parsed.sq_order.len() != references.len() {
        return Err(format!(
            "Replacement header has {} @SQ lines but the BAM reference dictionary has {} entries.",
            parsed.sq_order.len(),
            references.len()
        ));
    }

    if parsed.sq.len() != parsed.sq_order.len() {
        return Err("Replacement header contains duplicate @SQ names.".to_string());
    }

    for (index, (observed_name, expected_reference)) in
        parsed.sq_order.iter().zip(references.iter()).enumerate()
    {
        if observed_name != &expected_reference.name {
            return Err(format!(
                "Replacement header @SQ order mismatch at index {index}: expected {} but observed {}.",
                expected_reference.name, observed_name
            ));
        }

        let Some(sq) = parsed.sq.get(observed_name) else {
            return Err(format!(
                "Replacement header @SQ record for {} could not be resolved.",
                observed_name
            ));
        };
        match sq.length {
            Some(length) if length == expected_reference.length => {}
            Some(length) => {
                return Err(format!(
                    "Replacement header @SQ LN mismatch for {}: expected {} but observed {}.",
                    observed_name, expected_reference.length, length
                ));
            }
            None => {
                return Err(format!(
                    "Replacement header @SQ record for {} is missing LN.",
                    observed_name
                ));
            }
        }
    }

    Ok(())
}

fn validate_unique_read_groups(read_groups: &[ReadGroupRecord]) -> Result<(), String> {
    let mut seen = HashMap::new();
    for rg in read_groups {
        let Some(id) = &rg.id else {
            return Err("Header contains an @RG record without ID.".to_string());
        };
        if seen.insert(id.clone(), ()).is_some() {
            return Err(format!("Header contains duplicate @RG ID {id}."));
        }
    }
    Ok(())
}

fn validate_unique_programs(programs: &[ProgramRecord]) -> Result<(), String> {
    let mut seen = HashMap::new();
    for pg in programs {
        let Some(id) = &pg.id else {
            continue;
        };
        if seen.insert(id.clone(), ()).is_some() {
            return Err(format!("Header contains duplicate @PG ID {id}."));
        }
    }
    Ok(())
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

fn checked_bam_i32_from_usize(path: &Path, field: &str, value: usize) -> Result<i32, AppError> {
    let value = u32::try_from(value).map_err(|_| AppError::InvalidHeader {
        path: path.to_path_buf(),
        detail: format!("{field} {value} does not fit in a BAM signed 32-bit field."),
    })?;
    checked_bam_i32_from_u32(path, field, value)
}

fn checked_bam_i32_from_u32(path: &Path, field: &str, value: u32) -> Result<i32, AppError> {
    if value > MAX_BAM_I32_FIELD {
        return Err(AppError::InvalidHeader {
            path: path.to_path_buf(),
            detail: format!("{field} {value} does not fit in a BAM signed 32-bit field."),
        });
    }
    Ok(value as i32)
}

fn reconcile_reference_diagnostics(
    binary_references: &[BinaryReference],
    parsed: &ParsedSamHeader,
) -> Vec<ReferenceDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut binary_by_name = HashMap::new();
    for (index, reference) in binary_references.iter().enumerate() {
        binary_by_name.insert(reference.name.as_str(), (index, reference.length));
    }

    let mut text_indexes_by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for text_record in &parsed.sq_records {
        if let Some(name) = text_record.name.as_deref() {
            text_indexes_by_name
                .entry(name)
                .or_default()
                .push(text_record.index);
        } else {
            diagnostics.push(ReferenceDiagnostic {
                kind: "text_sq_missing_name",
                severity: "warning",
                binary_index: None,
                binary_name: None,
                binary_length: None,
                text_index: Some(text_record.index),
                text_name: None,
                text_length: text_record.length,
                message: format!(
                    "Textual @SQ record at index {} does not contain an SN field: {}",
                    text_record.index, text_record.raw_line
                ),
            });
        }
    }

    for (name, indexes) in &text_indexes_by_name {
        if indexes.len() > 1 {
            diagnostics.push(ReferenceDiagnostic {
                kind: "duplicate_text_sq",
                severity: "warning",
                binary_index: binary_by_name.get(name).map(|(index, _)| *index),
                binary_name: Some((*name).to_string()),
                binary_length: binary_by_name.get(name).map(|(_, length)| *length),
                text_index: indexes.first().copied(),
                text_name: Some((*name).to_string()),
                text_length: indexes
                    .first()
                    .and_then(|index| parsed.sq_records.get(*index))
                    .and_then(|record| record.length),
                message: format!(
                    "Textual header contains duplicate @SQ records for {name} at indexes {}.",
                    format_usize_list(indexes)
                ),
            });
        }
    }

    for (binary_index, binary_reference) in binary_references.iter().enumerate() {
        let text_indexes = text_indexes_by_name.get(binary_reference.name.as_str());
        match text_indexes.and_then(|indexes| indexes.first()).copied() {
            Some(text_index) => {
                let text_record = &parsed.sq_records[text_index];
                if text_index != binary_index {
                    diagnostics.push(ReferenceDiagnostic {
                        kind: "reference_order_mismatch",
                        severity: "warning",
                        binary_index: Some(binary_index),
                        binary_name: Some(binary_reference.name.clone()),
                        binary_length: Some(binary_reference.length),
                        text_index: Some(text_index),
                        text_name: text_record.name.clone(),
                        text_length: text_record.length,
                        message: format!(
                            "Binary reference {} appears at index {binary_index}, but the matching textual @SQ record appears at index {text_index}.",
                            binary_reference.name
                        ),
                    });
                }

                match text_record.length {
                    Some(text_length) if text_length != binary_reference.length => {
                        diagnostics.push(ReferenceDiagnostic {
                            kind: "reference_length_mismatch",
                            severity: "warning",
                            binary_index: Some(binary_index),
                            binary_name: Some(binary_reference.name.clone()),
                            binary_length: Some(binary_reference.length),
                            text_index: Some(text_index),
                            text_name: text_record.name.clone(),
                            text_length: Some(text_length),
                            message: format!(
                                "Binary reference {} has length {}, but the textual @SQ LN is {text_length}.",
                                binary_reference.name, binary_reference.length
                            ),
                        });
                    }
                    None => diagnostics.push(ReferenceDiagnostic {
                        kind: "text_sq_missing_length",
                        severity: "warning",
                        binary_index: Some(binary_index),
                        binary_name: Some(binary_reference.name.clone()),
                        binary_length: Some(binary_reference.length),
                        text_index: Some(text_index),
                        text_name: text_record.name.clone(),
                        text_length: None,
                        message: format!(
                            "Textual @SQ record for {} does not contain a parseable LN field.",
                            binary_reference.name
                        ),
                    }),
                    _ => {}
                }
            }
            None => diagnostics.push(ReferenceDiagnostic {
                kind: "missing_text_sq",
                severity: "warning",
                binary_index: Some(binary_index),
                binary_name: Some(binary_reference.name.clone()),
                binary_length: Some(binary_reference.length),
                text_index: None,
                text_name: None,
                text_length: None,
                message: format!(
                    "Binary reference {} at index {binary_index} has no matching textual @SQ record.",
                    binary_reference.name
                ),
            }),
        }

        if let Some(text_record) = parsed.sq_records.get(binary_index) {
            if text_record.name.as_deref() != Some(binary_reference.name.as_str()) {
                diagnostics.push(ReferenceDiagnostic {
                    kind: "reference_name_mismatch",
                    severity: "warning",
                    binary_index: Some(binary_index),
                    binary_name: Some(binary_reference.name.clone()),
                    binary_length: Some(binary_reference.length),
                    text_index: Some(text_record.index),
                    text_name: text_record.name.clone(),
                    text_length: text_record.length,
                    message: format!(
                        "Binary reference index {binary_index} is {}, but textual @SQ index {} is {}.",
                        binary_reference.name,
                        text_record.index,
                        text_record
                            .name
                            .as_deref()
                            .unwrap_or("<missing SN>")
                    ),
                });
            }
        }
    }

    for text_record in &parsed.sq_records {
        let Some(text_name) = text_record.name.as_deref() else {
            continue;
        };
        if !binary_by_name.contains_key(text_name) {
            diagnostics.push(ReferenceDiagnostic {
                kind: "extra_text_sq",
                severity: "warning",
                binary_index: None,
                binary_name: None,
                binary_length: None,
                text_index: Some(text_record.index),
                text_name: Some(text_name.to_string()),
                text_length: text_record.length,
                message: format!(
                    "Textual @SQ record {text_name} at index {} has no matching binary reference.",
                    text_record.index
                ),
            });
        }
    }

    diagnostics
}

fn format_usize_list(values: &[usize]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_len_prefixed(target: &mut Vec<u8>, bytes: &[u8]) {
    target.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    target.extend_from_slice(bytes);
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
    use std::{fs, path::Path};

    use super::{
        ReferenceHeaderFields, ReferenceRecord, parse_bam_header, parse_bam_header_from_reader,
        serialize_bam_header_checksum_domain, serialize_bam_header_payload,
        serialize_bam_header_view_payload,
    };
    use crate::{
        bam::{reader::BamReader, records::read_next_record_layout},
        bgzf::{BGZF_EOF_MARKER, test_support::build_bgzf_member},
        error::AppError,
        formats::bgzf::test_support::{
            build_bam_file_with_header, build_bam_file_with_header_and_records, build_light_record,
            write_temp_file,
        },
    };

    fn build_raw_bam_payload(header_bytes: impl AsRef<[u8]>) -> Vec<u8> {
        let mut bytes = build_bgzf_member(header_bytes.as_ref());
        bytes.extend_from_slice(&BGZF_EOF_MARKER);
        bytes
    }

    fn assert_truncated_detail(error: AppError, expected: &str) {
        match error {
            AppError::TruncatedFile { detail, .. } => {
                assert!(
                    detail.contains(expected),
                    "expected detail to contain {expected:?}, got {detail:?}"
                );
            }
            other => panic!("expected truncated file error, got {other:?}"),
        }
    }

    fn assert_invalid_header_detail(error: AppError, expected: &str) {
        match error {
            AppError::InvalidHeader { detail, .. } => {
                assert!(
                    detail.contains(expected),
                    "expected detail to contain {expected:?}, got {detail:?}"
                );
            }
            other => panic!("expected invalid header error, got {other:?}"),
        }
    }

    fn diagnostic_kinds(payload: &super::HeaderPayload) -> Vec<&str> {
        payload
            .header
            .reference_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind)
            .collect()
    }

    #[test]
    fn parses_empty_header_without_references() {
        let bytes = build_bam_file_with_header("", &[]);
        let path = write_temp_file("header-empty", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("empty BAM header should parse");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(payload.format, "BAM");
        assert!(payload.header.raw_header_text.is_empty());
        assert!(payload.header.references.is_empty());
        assert!(payload.header.reference_diagnostics.is_empty());
        assert!(payload.header.other_header_records.is_empty());
    }

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
        assert_eq!(payload.header.raw_header_text, header_text);
        assert_eq!(payload.header.hd.version.as_deref(), Some("1.6"));
        assert_eq!(payload.header.hd.sort_order.as_deref(), Some("coordinate"));
        assert_eq!(payload.header.hd.group_order.as_deref(), Some("query"));
        assert_eq!(payload.header.references.len(), 2);
        assert_eq!(payload.header.references[0].name, "chr1");
        assert_eq!(payload.header.references[0].length, 248_956_422);
        assert_eq!(payload.header.references[0].index, 0);
        assert_eq!(payload.header.references[1].name, "chr2");
        assert_eq!(payload.header.references[1].length, 242_193_529);
        assert_eq!(payload.header.references[1].index, 1);
        assert!(payload.header.reference_diagnostics.is_empty());
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
        assert_eq!(
            payload.header.other_header_records[0].raw_line,
            "@XY\tZZ:custom"
        );
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
        assert!(diagnostic_kinds(&payload).contains(&"reference_length_mismatch"));
        let diagnostic = payload
            .header
            .reference_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == "reference_length_mismatch")
            .expect("length mismatch diagnostic should be present");
        assert_eq!(diagnostic.binary_length, Some(456));
        assert_eq!(diagnostic.text_length, Some(123));
    }

    #[test]
    fn reports_missing_text_sq_without_rewriting_binary_reference() {
        let bytes = build_bam_file_with_header("@HD\tVN:1.6\n", &[("chr1", 100)]);
        let path = write_temp_file("header-missing-text-sq", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse with warning");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(payload.header.references[0].name, "chr1");
        assert_eq!(payload.header.references[0].length, 100);
        assert!(diagnostic_kinds(&payload).contains(&"missing_text_sq"));
    }

    #[test]
    fn reports_extra_text_sq_record() {
        let header_text = "@SQ\tSN:chr1\tLN:100\n@SQ\tSN:chr2\tLN:200\n";
        let bytes = build_bam_file_with_header(header_text, &[("chr1", 100)]);
        let path = write_temp_file("header-extra-text-sq", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse with warning");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(payload.header.references.len(), 1);
        let diagnostic = payload
            .header
            .reference_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == "extra_text_sq")
            .expect("extra @SQ diagnostic should be present");
        assert_eq!(diagnostic.text_name.as_deref(), Some("chr2"));
        assert_eq!(diagnostic.text_index, Some(1));
    }

    #[test]
    fn reports_name_mismatch_at_same_reference_index() {
        let header_text = "@SQ\tSN:chr2\tLN:100\n";
        let bytes = build_bam_file_with_header(header_text, &[("chr1", 100)]);
        let path = write_temp_file("header-name-mismatch", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse with warning");
        fs::remove_file(path).expect("fixture should be removed");

        let kinds = diagnostic_kinds(&payload);
        assert!(kinds.contains(&"reference_name_mismatch"));
        assert!(kinds.contains(&"missing_text_sq"));
        assert!(kinds.contains(&"extra_text_sq"));
        assert_eq!(payload.header.references[0].name, "chr1");
    }

    #[test]
    fn reports_text_sq_order_mismatch() {
        let header_text = "@SQ\tSN:chr2\tLN:200\n@SQ\tSN:chr1\tLN:100\n";
        let bytes = build_bam_file_with_header(header_text, &[("chr1", 100), ("chr2", 200)]);
        let path = write_temp_file("header-order-mismatch", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse with warning");
        fs::remove_file(path).expect("fixture should be removed");

        let kinds = diagnostic_kinds(&payload);
        assert!(kinds.contains(&"reference_order_mismatch"));
        assert!(kinds.contains(&"reference_name_mismatch"));
        assert_eq!(payload.header.references[0].name, "chr1");
        assert_eq!(payload.header.references[1].name, "chr2");
    }

    #[test]
    fn reports_duplicate_text_sq_records() {
        let header_text = "@SQ\tSN:chr1\tLN:100\n@SQ\tSN:chr1\tLN:100\n";
        let bytes = build_bam_file_with_header(header_text, &[("chr1", 100)]);
        let path = write_temp_file("header-duplicate-text-sq", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse with warning");
        fs::remove_file(path).expect("fixture should be removed");

        let diagnostic = payload
            .header
            .reference_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == "duplicate_text_sq")
            .expect("duplicate @SQ diagnostic should be present");
        assert_eq!(diagnostic.text_name.as_deref(), Some("chr1"));
        assert!(diagnostic.message.contains("0, 1"));
    }

    #[test]
    fn reports_text_sq_records_without_names() {
        let header_text = "@SQ\tLN:100\n";
        let bytes = build_bam_file_with_header(header_text, &[]);
        let path = write_temp_file("header-text-sq-missing-name", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse with warning");
        fs::remove_file(path).expect("fixture should be removed");

        let diagnostic = payload
            .header
            .reference_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == "text_sq_missing_name")
            .expect("missing SN diagnostic should be present");
        assert_eq!(diagnostic.text_index, Some(0));
        assert!(diagnostic.message.contains("@SQ"));
    }

    #[test]
    fn reports_text_sq_records_without_parseable_lengths() {
        let header_text = "@SQ\tSN:chr1\n";
        let bytes = build_bam_file_with_header(header_text, &[("chr1", 100)]);
        let path = write_temp_file("header-text-sq-missing-length", "bam", &bytes);

        let payload = parse_bam_header(&path).expect("header should parse with warning");
        fs::remove_file(path).expect("fixture should be removed");

        let diagnostic = payload
            .header
            .reference_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == "text_sq_missing_length")
            .expect("missing LN diagnostic should be present");
        assert_eq!(diagnostic.binary_name.as_deref(), Some("chr1"));
        assert_eq!(diagnostic.text_index, Some(0));
    }

    #[test]
    fn bam_header_view_serialization_is_deterministic_and_parseable() {
        let header_text = concat!(
            "@HD\tVN:1.6\tSO:coordinate\n",
            "@SQ\tSN:chr1\tLN:100\tM5:abc\n",
            "@SQ\tSN:chr2\tLN:200\tUR:file://ref.fa\n",
            "@CO\tdeterministic serialization\n"
        );
        let bytes = build_bam_file_with_header(header_text, &[("chr1", 100), ("chr2", 200)]);
        let path = write_temp_file("header-deterministic-source", "bam", &bytes);
        let parsed = parse_bam_header(&path).expect("source header should parse");

        let first = serialize_bam_header_view_payload(&path, &parsed.header)
            .expect("header view should serialize");
        let second = serialize_bam_header_view_payload(&path, &parsed.header)
            .expect("header view should serialize repeatedly");
        assert_eq!(first, second);

        let reserialized_path = write_temp_file(
            "header-deterministic-reserialized",
            "bam",
            &build_raw_bam_payload(&first),
        );
        let reparsed =
            parse_bam_header(&reserialized_path).expect("serialized header should parse");
        fs::remove_file(path).expect("fixture should be removed");
        fs::remove_file(reserialized_path).expect("fixture should be removed");

        assert_eq!(
            reparsed.header.raw_header_text,
            parsed.header.raw_header_text
        );
        assert_eq!(reparsed.header.references.len(), 2);
        assert_eq!(reparsed.header.references[0].name, "chr1");
        assert_eq!(reparsed.header.references[0].length, 100);
        assert_eq!(reparsed.header.references[1].name, "chr2");
        assert_eq!(reparsed.header.references[1].length, 200);
        assert!(reparsed.header.reference_diagnostics.is_empty());
    }

    #[test]
    fn serialized_binary_references_have_exactly_one_nul_terminator() {
        let header_text = "@SQ\tSN:chr1\tLN:100\n";
        let reference = ReferenceRecord {
            name: "chr1".to_string(),
            length: 100,
            index: 0,
            header_fields: ReferenceHeaderFields::default(),
            text_header_length: None,
        };
        let payload = serialize_bam_header_payload(
            Path::new("header-nul-terminator.bam"),
            header_text,
            &[reference],
        )
        .expect("header should serialize");

        let mut offset = 4;
        offset += 4;
        offset += header_text.len();
        let n_ref = i32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let l_name = i32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let name_bytes = &payload[offset..offset + l_name as usize];

        assert_eq!(n_ref, 1);
        assert_eq!(l_name, 5);
        assert_eq!(name_bytes, b"chr1\0");
        assert_eq!(name_bytes.iter().filter(|byte| **byte == 0).count(), 1);
    }

    #[test]
    fn checksum_header_domain_serialization_is_deterministic() {
        let bytes = build_bam_file_with_header("@SQ\tSN:chr1\tLN:100\n", &[("chr1", 100)]);
        let path = write_temp_file("header-checksum-domain", "bam", &bytes);
        let parsed = parse_bam_header(&path).expect("header should parse");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(
            serialize_bam_header_checksum_domain(&parsed),
            serialize_bam_header_checksum_domain(&parsed)
        );
    }

    #[test]
    fn parser_leaves_reader_at_first_alignment_record() {
        let record = build_light_record(0, 42, "read1", 0);
        let bytes = build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:100\n",
            &[("chr1", 100)],
            &[record],
        );
        let path = write_temp_file("header-reader-position", "bam", &bytes);
        let mut reader = BamReader::open(&path).expect("BAM should open");

        let header = parse_bam_header_from_reader(&mut reader).expect("header should parse");
        let record = read_next_record_layout(&mut reader)
            .expect("record read should succeed")
            .expect("record should exist after header");
        fs::remove_file(path).expect("fixture should be removed");

        assert_eq!(header.header.references[0].name, "chr1");
        assert_eq!(record.read_name, "read1");
        assert_eq!(record.pos, 42);
    }

    #[test]
    fn reports_truncated_header_text_with_specific_context() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&8_i32.to_le_bytes());
        payload.extend_from_slice(b"@HD\n");
        let path = write_temp_file(
            "header-truncated-text",
            "bam",
            &build_raw_bam_payload(payload),
        );

        let error = parse_bam_header(&path).expect_err("truncated text should fail");
        fs::remove_file(path).expect("fixture should be removed");

        assert_truncated_detail(error, "header text");
    }

    #[test]
    fn reports_truncated_reference_count_with_specific_context() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&0_i32.to_le_bytes());
        let path = write_temp_file(
            "header-truncated-nref",
            "bam",
            &build_raw_bam_payload(payload),
        );

        let error = parse_bam_header(&path).expect_err("truncated n_ref should fail");
        fs::remove_file(path).expect("fixture should be removed");

        assert_truncated_detail(error, "reference count");
    }

    #[test]
    fn reports_truncated_reference_name_with_specific_context() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&5_i32.to_le_bytes());
        payload.extend_from_slice(b"chr");
        let path = write_temp_file(
            "header-truncated-reference-name",
            "bam",
            &build_raw_bam_payload(payload),
        );

        let error = parse_bam_header(&path).expect_err("truncated reference name should fail");
        fs::remove_file(path).expect("fixture should be removed");

        assert_truncated_detail(error, "reference name");
    }

    #[test]
    fn rejects_reference_name_without_nul_terminator() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&4_i32.to_le_bytes());
        payload.extend_from_slice(b"chr1");
        payload.extend_from_slice(&10_i32.to_le_bytes());
        let path = write_temp_file(
            "header-reference-missing-nul",
            "bam",
            &build_raw_bam_payload(payload),
        );

        let error = parse_bam_header(&path).expect_err("missing NUL should fail");
        fs::remove_file(path).expect("fixture should be removed");

        assert_invalid_header_detail(error, "NUL-terminated");
    }

    #[test]
    fn rejects_reference_name_with_interior_nul() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&6_i32.to_le_bytes());
        payload.extend_from_slice(b"ch\0r1\0");
        payload.extend_from_slice(&10_i32.to_le_bytes());
        let path = write_temp_file(
            "header-reference-interior-nul",
            "bam",
            &build_raw_bam_payload(payload),
        );

        let error = parse_bam_header(&path).expect_err("interior NUL should fail");
        fs::remove_file(path).expect("fixture should be removed");

        assert_invalid_header_detail(error, "interior NUL");
    }

    #[test]
    fn reports_truncated_reference_length_with_specific_context() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BAM\x01");
        payload.extend_from_slice(&0_i32.to_le_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&5_i32.to_le_bytes());
        payload.extend_from_slice(b"chr1\0");
        payload.extend_from_slice(&10_i16.to_le_bytes());
        let path = write_temp_file(
            "header-truncated-reference-length",
            "bam",
            &build_raw_bam_payload(payload),
        );

        let error = parse_bam_header(&path).expect_err("truncated reference length should fail");
        fs::remove_file(path).expect("fixture should be removed");

        assert_truncated_detail(error, "reference length");
    }

    #[test]
    fn serialization_rejects_reference_lengths_outside_bam_i32_range() {
        let error = serialize_bam_header_payload(
            Path::new("oversized-reference.bam"),
            "@SQ\tSN:chr1\tLN:2147483648\n",
            &[ReferenceRecord {
                name: "chr1".to_string(),
                length: i32::MAX as u32 + 1,
                index: 0,
                header_fields: ReferenceHeaderFields::default(),
                text_header_length: None,
            }],
        )
        .expect_err("out-of-range BAM reference length should fail");

        match error {
            AppError::InvalidHeader { path, detail } => {
                assert_eq!(path, Path::new("oversized-reference.bam"));
                assert!(detail.contains("BAM reference length 2147483648"));
                assert!(detail.contains("signed 32-bit field"));
            }
            other => panic!("expected invalid_header error, got {other:?}"),
        }
    }
}
