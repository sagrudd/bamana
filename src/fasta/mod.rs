use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::error::AppError;

pub fn count_fasta_records(path: &Path) -> Result<u64, AppError> {
    count_fasta_records_with_label(path, path)
}

pub fn count_fasta_records_with_label(path: &Path, label: &Path) -> Result<u64, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(label, error))?;
    count_fasta_reader(BufReader::new(file), label)
}

fn count_fasta_reader<R: BufRead>(reader: R, path: &Path) -> Result<u64, AppError> {
    let mut records = 0_u64;
    let mut seen_header = false;

    for line_result in reader.lines() {
        let line = line_result.map_err(|error| AppError::from_io(path, error))?;
        if line.is_empty() {
            continue;
        }

        if line.starts_with('>') {
            records += 1;
            seen_header = true;
            continue;
        }

        if !seen_header {
            return Err(AppError::ParseError {
                path: path.to_path_buf(),
                detail: "FASTA sequence data was encountered before the first header line."
                    .to_string(),
            });
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::count_fasta_records;

    #[test]
    fn counts_fasta_entries() {
        let path =
            std::env::temp_dir().join(format!("bamana-fasta-count-{}.fa", std::process::id()));
        fs::write(&path, ">chr1\nACGT\n>chr2\nTTAA\n").expect("fasta should write");

        let count = count_fasta_records(&path).expect("fasta should count");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(count, 2);
    }

    #[test]
    fn rejects_sequence_data_before_header() {
        let path =
            std::env::temp_dir().join(format!("bamana-fasta-invalid-{}.fa", std::process::id()));
        fs::write(&path, "ACGT\n>chr1\nTTAA\n").expect("fasta should write");

        let error = count_fasta_records(&path).expect_err("fasta should fail");
        fs::remove_file(path).expect("fixture should be removable");

        assert_eq!(error.to_json_error().code, "parse_error");
    }
}
