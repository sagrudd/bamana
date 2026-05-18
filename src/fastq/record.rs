#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastqRecord {
    pub raw_header_line: String,
    pub read_name: String,
    pub sequence: String,
    pub plus_line: String,
    pub quality: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastqIdentityBasis {
    Qname,
    QnameSeq,
    FullRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastqRecordValidationError {
    detail: String,
}

impl FastqRecordValidationError {
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FastqRecordView<'a> {
    raw_header_line: &'a str,
    read_name: &'a str,
    sequence: &'a str,
    plus_line: &'a str,
    quality: &'a str,
}

impl FastqRecord {
    pub fn from_lines(
        raw_header_line: String,
        sequence: String,
        plus_line: String,
        quality: String,
    ) -> Result<Self, FastqRecordValidationError> {
        let read_name = {
            let view =
                FastqRecordView::from_lines(&raw_header_line, &sequence, &plus_line, &quality)?;
            view.read_name().to_string()
        };

        Ok(Self {
            raw_header_line,
            read_name,
            sequence,
            plus_line,
            quality,
        })
    }

    pub fn view(&self) -> FastqRecordView<'_> {
        FastqRecordView {
            raw_header_line: &self.raw_header_line,
            read_name: &self.read_name,
            sequence: &self.sequence,
            plus_line: &self.plus_line,
            quality: &self.quality,
        }
    }

    pub fn identity_bytes(&self, basis: FastqIdentityBasis) -> Vec<u8> {
        self.view().identity_bytes(basis)
    }
}

impl<'a> FastqRecordView<'a> {
    pub fn from_lines(
        raw_header_line: &'a str,
        sequence: &'a str,
        plus_line: &'a str,
        quality: &'a str,
    ) -> Result<Self, FastqRecordValidationError> {
        if !raw_header_line.starts_with('@') {
            return Err(FastqRecordValidationError {
                detail: "FASTQ record header line did not start with '@'.".to_string(),
            });
        }
        if !plus_line.starts_with('+') {
            return Err(FastqRecordValidationError {
                detail: "FASTQ record plus line did not start with '+'.".to_string(),
            });
        }
        if sequence.len() != quality.len() {
            return Err(FastqRecordValidationError {
                detail: format!(
                    "FASTQ sequence and quality lengths differed ({} vs {}).",
                    sequence.len(),
                    quality.len()
                ),
            });
        }

        let read_name =
            parse_read_name(raw_header_line).ok_or_else(|| FastqRecordValidationError {
                detail: "FASTQ record header did not contain a usable read name.".to_string(),
            })?;

        Ok(Self {
            raw_header_line,
            read_name,
            sequence,
            plus_line,
            quality,
        })
    }

    pub fn raw_header_line(&self) -> &'a str {
        self.raw_header_line
    }

    pub fn read_name(&self) -> &'a str {
        self.read_name
    }

    pub fn sequence(&self) -> &'a str {
        self.sequence
    }

    pub fn plus_line(&self) -> &'a str {
        self.plus_line
    }

    pub fn quality(&self) -> &'a str {
        self.quality
    }

    pub fn lines(&self) -> [&'a str; 4] {
        [
            self.raw_header_line,
            self.sequence,
            self.plus_line,
            self.quality,
        ]
    }

    pub fn identity_bytes(&self, basis: FastqIdentityBasis) -> Vec<u8> {
        match basis {
            FastqIdentityBasis::Qname => self.read_name.as_bytes().to_vec(),
            FastqIdentityBasis::QnameSeq => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(self.read_name.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(self.sequence.as_bytes());
                bytes
            }
            FastqIdentityBasis::FullRecord => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(self.raw_header_line.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(self.sequence.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(self.plus_line.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(self.quality.as_bytes());
                bytes
            }
        }
    }
}

pub(crate) fn parse_read_name(header_line: &str) -> Option<&str> {
    header_line
        .strip_prefix('@')?
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{FastqIdentityBasis, FastqRecord, FastqRecordView};

    #[test]
    fn owned_record_preserves_header_comment_and_plus_comment() {
        let record = FastqRecord::from_lines(
            "@read1 run=42".to_string(),
            "ACGTN".to_string(),
            "+preserved plus comment".to_string(),
            "!!!!!".to_string(),
        )
        .expect("record should validate");

        assert_eq!(record.read_name, "read1");
        assert_eq!(record.raw_header_line, "@read1 run=42");
        assert_eq!(record.plus_line, "+preserved plus comment");
        assert_eq!(record.sequence, "ACGTN");
        assert_eq!(record.quality, "!!!!!");
    }

    #[test]
    fn borrowed_view_preserves_representative_payload_without_allocation() {
        let view = FastqRecordView::from_lines("@read2", "ACGTNRY", "+", "!#$%&'(")
            .expect("view should validate");

        assert_eq!(view.read_name(), "read2");
        assert_eq!(view.raw_header_line(), "@read2");
        assert_eq!(view.sequence(), "ACGTNRY");
        assert_eq!(view.plus_line(), "+");
        assert_eq!(view.quality(), "!#$%&'(");
        assert_eq!(view.lines(), ["@read2", "ACGTNRY", "+", "!#$%&'("]);
    }

    #[test]
    fn header_with_empty_comment_still_has_usable_read_name() {
        let view = FastqRecordView::from_lines("@read3   ", "NN", "+comment", "##")
            .expect("view should validate");

        assert_eq!(view.read_name(), "read3");
        assert_eq!(view.plus_line(), "+comment");
    }

    #[test]
    fn identity_bytes_are_defined_by_the_fastq_contract() {
        let record = FastqRecord::from_lines(
            "@read4 comment".to_string(),
            "AC".to_string(),
            "+plus".to_string(),
            "!!".to_string(),
        )
        .expect("record should validate");

        assert_eq!(record.identity_bytes(FastqIdentityBasis::Qname), b"read4");
        assert_eq!(
            record.identity_bytes(FastqIdentityBasis::QnameSeq),
            b"read4\0AC"
        );
        assert_eq!(
            record.identity_bytes(FastqIdentityBasis::FullRecord),
            b"@read4 comment\0AC\0+plus\0!!"
        );
    }

    #[test]
    fn invalid_record_view_reports_contract_errors() {
        let error = FastqRecordView::from_lines("read5", "A", "+", "!")
            .expect_err("header marker should be required");
        assert_eq!(
            error.detail(),
            "FASTQ record header line did not start with '@'."
        );

        let error = FastqRecordView::from_lines("@read5", "AA", "+", "!")
            .expect_err("sequence and quality lengths should match");
        assert_eq!(
            error.detail(),
            "FASTQ sequence and quality lengths differed (2 vs 1)."
        );
    }
}
