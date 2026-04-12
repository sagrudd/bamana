use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::{
    bam::{
        header::{ReferenceHeaderFields, ReferenceRecord},
        records::{
            RecordLayout, encode_bam_qualities, encode_bam_sequence, missing_quality_scores,
            reg2bin,
        },
    },
    error::AppError,
};

#[derive(Debug)]
pub struct ParsedSamFile {
    pub raw_header_text: String,
    pub references: Vec<ReferenceRecord>,
    pub records: Vec<RecordLayout>,
}

pub fn read_sam_file(path: &Path) -> Result<ParsedSamFile, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let reader = BufReader::new(file);

    let mut header_lines = Vec::new();
    let mut records = Vec::new();
    let mut seen_record = false;

    let mut references = Vec::new();
    let mut ref_name_to_id = HashMap::new();

    for line_result in reader.lines() {
        let line = line_result.map_err(|error| AppError::from_io(path, error))?;
        if line.is_empty() {
            continue;
        }

        if line.starts_with('@') {
            if seen_record {
                return Err(AppError::InvalidHeader {
                    path: path.to_path_buf(),
                    detail: "SAM header lines were encountered after alignment records began."
                        .to_string(),
                });
            }
            if line.starts_with("@SQ") {
                let reference = parse_sq_line(path, &line, references.len())?;
                ref_name_to_id.insert(reference.name.clone(), reference.index as i32);
                references.push(reference);
            }
            header_lines.push(line);
            continue;
        }

        seen_record = true;
        records.push(parse_alignment_line(path, &line, &ref_name_to_id)?);
    }

    let raw_header_text = if header_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", header_lines.join("\n"))
    };

    Ok(ParsedSamFile {
        raw_header_text,
        references,
        records,
    })
}

fn parse_sq_line(path: &Path, line: &str, index: usize) -> Result<ReferenceRecord, AppError> {
    let mut name = None;
    let mut length = None;
    let mut fields = ReferenceHeaderFields::default();

    for field in line.split('\t').skip(1) {
        let Some((tag, value)) = field.split_once(':') else {
            continue;
        };
        match tag {
            "SN" => name = Some(value.to_string()),
            "LN" => {
                length = Some(value.parse::<u32>().map_err(|_| AppError::InvalidHeader {
                    path: path.to_path_buf(),
                    detail: format!("SAM @SQ length was not a valid unsigned integer: {value}"),
                })?)
            }
            "M5" => fields.m5 = Some(value.to_string()),
            "UR" => fields.ur = Some(value.to_string()),
            "AS" => fields.assembly = Some(value.to_string()),
            "SP" => fields.species = Some(value.to_string()),
            "TP" => fields.topology = Some(value.to_string()),
            _ => {}
        }
    }

    Ok(ReferenceRecord {
        name: name.ok_or_else(|| AppError::InvalidHeader {
            path: path.to_path_buf(),
            detail: "SAM @SQ line was missing SN.".to_string(),
        })?,
        length: length.ok_or_else(|| AppError::InvalidHeader {
            path: path.to_path_buf(),
            detail: "SAM @SQ line was missing LN.".to_string(),
        })?,
        index,
        header_fields: fields,
        text_header_length: None,
    })
}

fn parse_alignment_line(
    path: &Path,
    line: &str,
    ref_name_to_id: &HashMap<String, i32>,
) -> Result<RecordLayout, AppError> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() < 11 {
        return Err(AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "SAM alignment line did not contain the required 11 fields.".to_string(),
        });
    }

    let read_name = fields[0].to_string();
    if read_name.is_empty() || read_name == "*" {
        return Err(AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "SAM alignment line contained an empty or missing QNAME.".to_string(),
        });
    }

    let flags = parse_number::<u16>(path, "FLAG", fields[1])?;
    let ref_id = parse_ref_id(path, fields[2], ref_name_to_id)?;
    let pos = parse_position(path, fields[3])?;
    let mapping_quality = parse_number::<u8>(path, "MAPQ", fields[4])?;
    let cigar = parse_cigar(path, fields[5])?;
    let next_ref_id = parse_next_ref_id(path, fields[6], fields[2], ref_name_to_id)?;
    let next_pos = parse_position(path, fields[7])?;
    let tlen = parse_number::<i32>(path, "TLEN", fields[8])?;

    let sequence = fields[9];
    let qualities = fields[10];
    let l_seq = if sequence == "*" { 0 } else { sequence.len() };
    let sequence_bytes = if sequence == "*" {
        Vec::new()
    } else {
        encode_bam_sequence(sequence).map_err(|detail| AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail,
        })?
    };
    let quality_bytes = match qualities {
        "*" => missing_quality_scores(l_seq),
        _ => {
            if qualities.len() != l_seq {
                return Err(AppError::InvalidRecord {
                    path: path.to_path_buf(),
                    detail: format!(
                        "SAM QUAL length {} did not match SEQ length {l_seq}.",
                        qualities.len()
                    ),
                });
            }
            encode_bam_qualities(qualities).map_err(|detail| AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail,
            })?
        }
    };
    let aux_bytes = parse_aux_fields(path, &fields[11..])?;

    let alignment_end = if ref_id < 0 || pos < 0 {
        pos
    } else {
        pos + cigar.reference_span.max(1)
    };
    let bin = if ref_id < 0 || pos < 0 {
        4680
    } else {
        reg2bin(pos, alignment_end)
    };
    let block_size = 32
        + read_name.len()
        + 1
        + cigar.bytes.len()
        + sequence_bytes.len()
        + quality_bytes.len()
        + aux_bytes.len();

    Ok(RecordLayout {
        block_size,
        ref_id,
        pos,
        bin,
        next_ref_id,
        next_pos,
        tlen,
        flags,
        mapping_quality,
        n_cigar_op: cigar.operation_count,
        l_seq,
        read_name,
        cigar_bytes: cigar.bytes,
        sequence_bytes,
        quality_bytes,
        aux_bytes,
    })
}

fn parse_ref_id(
    path: &Path,
    rname: &str,
    ref_name_to_id: &HashMap<String, i32>,
) -> Result<i32, AppError> {
    match rname {
        "*" => Ok(-1),
        value => ref_name_to_id
            .get(value)
            .copied()
            .ok_or_else(|| AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: format!("SAM RNAME {value} was not present in the parsed header."),
            }),
    }
}

fn parse_next_ref_id(
    path: &Path,
    rnext: &str,
    rname: &str,
    ref_name_to_id: &HashMap<String, i32>,
) -> Result<i32, AppError> {
    match rnext {
        "*" => Ok(-1),
        "=" => parse_ref_id(path, rname, ref_name_to_id),
        value => parse_ref_id(path, value, ref_name_to_id),
    }
}

fn parse_position(path: &Path, value: &str) -> Result<i32, AppError> {
    let position = parse_number::<i32>(path, "POS", value)?;
    if position == 0 {
        Ok(-1)
    } else if position > 0 {
        Ok(position - 1)
    } else {
        Err(AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: format!("SAM position field was negative: {value}"),
        })
    }
}

fn parse_number<T>(path: &Path, field_name: &str, value: &str) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| AppError::InvalidRecord {
        path: path.to_path_buf(),
        detail: format!("SAM field {field_name} did not parse cleanly: {value}"),
    })
}

struct ParsedCigar {
    bytes: Vec<u8>,
    operation_count: usize,
    reference_span: i32,
}

fn parse_cigar(path: &Path, cigar: &str) -> Result<ParsedCigar, AppError> {
    if cigar == "*" {
        return Ok(ParsedCigar {
            bytes: Vec::new(),
            operation_count: 0,
            reference_span: 0,
        });
    }

    let mut length_buffer = String::new();
    let mut bytes = Vec::new();
    let mut operation_count = 0_usize;
    let mut reference_span = 0_i32;

    for character in cigar.chars() {
        if character.is_ascii_digit() {
            length_buffer.push(character);
            continue;
        }

        if length_buffer.is_empty() {
            return Err(AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: format!("SAM CIGAR contained an operation without a length: {cigar}"),
            });
        }
        let length = length_buffer
            .parse::<u32>()
            .map_err(|_| AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: format!("SAM CIGAR operation length was invalid: {cigar}"),
            })?;
        length_buffer.clear();

        let opcode = cigar_opcode(character).ok_or_else(|| AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: format!("SAM CIGAR contained unsupported operation {character}."),
        })?;
        if matches!(character, 'M' | 'D' | 'N' | '=' | 'X') {
            reference_span += length as i32;
        }
        bytes.extend_from_slice(&((length << 4) | u32::from(opcode)).to_le_bytes());
        operation_count += 1;
    }

    if !length_buffer.is_empty() {
        return Err(AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: format!("SAM CIGAR ended with a dangling length: {cigar}"),
        });
    }

    Ok(ParsedCigar {
        bytes,
        operation_count,
        reference_span,
    })
}

fn cigar_opcode(character: char) -> Option<u8> {
    match character {
        'M' => Some(0),
        'I' => Some(1),
        'D' => Some(2),
        'N' => Some(3),
        'S' => Some(4),
        'H' => Some(5),
        'P' => Some(6),
        '=' => Some(7),
        'X' => Some(8),
        'B' => Some(9),
        _ => None,
    }
}

fn parse_aux_fields(path: &Path, fields: &[&str]) -> Result<Vec<u8>, AppError> {
    let mut aux = Vec::new();
    for field in fields {
        let parts = field.splitn(3, ':').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: format!("SAM auxiliary field did not have TAG:TYPE:VALUE form: {field}"),
            });
        }
        let tag = parts[0].as_bytes();
        if tag.len() != 2 {
            return Err(AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: format!("SAM auxiliary tag was not two characters: {}", parts[0]),
            });
        }
        aux.extend_from_slice(tag);
        let type_code =
            parts[1]
                .as_bytes()
                .first()
                .copied()
                .ok_or_else(|| AppError::InvalidRecord {
                    path: path.to_path_buf(),
                    detail: format!("SAM auxiliary field was missing a type code: {field}"),
                })?;
        aux.push(type_code);
        encode_aux_value(path, type_code, parts[2], &mut aux)?;
    }
    Ok(aux)
}

fn encode_aux_value(
    path: &Path,
    type_code: u8,
    value: &str,
    aux: &mut Vec<u8>,
) -> Result<(), AppError> {
    match type_code {
        b'A' => {
            let byte = value
                .bytes()
                .next()
                .ok_or_else(|| AppError::InvalidRecord {
                    path: path.to_path_buf(),
                    detail: "SAM auxiliary A field was empty.".to_string(),
                })?;
            aux.push(byte);
        }
        b'c' => aux.push(parse_aux_number::<i8>(path, type_code, value)? as u8),
        b'C' => aux.push(parse_aux_number::<u8>(path, type_code, value)?),
        b's' => {
            aux.extend_from_slice(&parse_aux_number::<i16>(path, type_code, value)?.to_le_bytes())
        }
        b'S' => {
            aux.extend_from_slice(&parse_aux_number::<u16>(path, type_code, value)?.to_le_bytes())
        }
        b'i' => {
            aux.extend_from_slice(&parse_aux_number::<i32>(path, type_code, value)?.to_le_bytes())
        }
        b'I' => {
            aux.extend_from_slice(&parse_aux_number::<u32>(path, type_code, value)?.to_le_bytes())
        }
        b'f' => {
            aux.extend_from_slice(&parse_aux_number::<f32>(path, type_code, value)?.to_le_bytes())
        }
        b'Z' => {
            aux.extend_from_slice(value.as_bytes());
            aux.push(0);
        }
        b'H' => {
            if value.len() % 2 != 0 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(AppError::InvalidRecord {
                    path: path.to_path_buf(),
                    detail: format!("SAM auxiliary H field was not valid hex: {value}"),
                });
            }
            aux.extend_from_slice(value.as_bytes());
            aux.push(0);
        }
        b'B' => encode_array_aux(path, value, aux)?,
        _ => {
            return Err(AppError::InvalidRecord {
                path: path.to_path_buf(),
                detail: format!(
                    "SAM auxiliary type {} is not supported in this slice.",
                    type_code as char
                ),
            });
        }
    }
    Ok(())
}

fn encode_array_aux(path: &Path, value: &str, aux: &mut Vec<u8>) -> Result<(), AppError> {
    let mut parts = value.split(',');
    let subtype = parts
        .next()
        .and_then(|item| item.as_bytes().first().copied())
        .ok_or_else(|| AppError::InvalidRecord {
            path: path.to_path_buf(),
            detail: "SAM B-array auxiliary field was missing a subtype.".to_string(),
        })?;
    aux.push(subtype);

    let values = parts.collect::<Vec<_>>();
    aux.extend_from_slice(&(values.len() as i32).to_le_bytes());

    for value in values {
        match subtype {
            b'c' => aux.push(parse_aux_number::<i8>(path, subtype, value)? as u8),
            b'C' => aux.push(parse_aux_number::<u8>(path, subtype, value)?),
            b's' => {
                aux.extend_from_slice(&parse_aux_number::<i16>(path, subtype, value)?.to_le_bytes())
            }
            b'S' => {
                aux.extend_from_slice(&parse_aux_number::<u16>(path, subtype, value)?.to_le_bytes())
            }
            b'i' => {
                aux.extend_from_slice(&parse_aux_number::<i32>(path, subtype, value)?.to_le_bytes())
            }
            b'I' => {
                aux.extend_from_slice(&parse_aux_number::<u32>(path, subtype, value)?.to_le_bytes())
            }
            b'f' => {
                aux.extend_from_slice(&parse_aux_number::<f32>(path, subtype, value)?.to_le_bytes())
            }
            _ => {
                return Err(AppError::InvalidRecord {
                    path: path.to_path_buf(),
                    detail: format!(
                        "SAM B-array subtype {} is not supported in this slice.",
                        subtype as char
                    ),
                });
            }
        }
    }

    Ok(())
}

fn parse_aux_number<T>(path: &Path, type_code: u8, value: &str) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| AppError::InvalidRecord {
        path: path.to_path_buf(),
        detail: format!(
            "SAM auxiliary field of type {} did not parse cleanly: {value}",
            type_code as char
        ),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::read_sam_file;

    #[test]
    fn parses_minimal_sam_with_header_and_aux() {
        let path = std::env::temp_dir().join(format!("bamana-sam-{}.sam", std::process::id()));
        fs::write(
            &path,
            concat!(
                "@HD\tVN:1.6\tSO:coordinate\n",
                "@SQ\tSN:chr1\tLN:10\n",
                "read1\t0\tchr1\t1\t60\t4M\t*\t0\t0\tACGT\t!!!!\tNM:i:1\tRG:Z:rg1\n"
            ),
        )
        .expect("sam fixture should write");

        let parsed = read_sam_file(&path).expect("sam should parse");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(parsed.references.len(), 1);
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].read_name, "read1");
        assert_eq!(parsed.records[0].ref_id, 0);
        assert_eq!(parsed.records[0].pos, 0);
        assert_eq!(parsed.records[0].n_cigar_op, 1);
    }
}
