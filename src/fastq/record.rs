#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastqRecord {
    pub raw_header_line: String,
    pub read_name: String,
    pub sequence: String,
    pub plus_line: String,
    pub quality: String,
}

pub(crate) fn parse_read_name(header_line: &str) -> Option<String> {
    header_line
        .strip_prefix('@')?
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
