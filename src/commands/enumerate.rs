use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

use serde::Serialize;

use crate::{
    error::AppError,
    fasta::count_fasta_records,
    fastq::{count_fastq_records, gzi::ensure_fastq_gzi},
    formats::probe::{DetectedFormat, probe_path},
    ingest::sam::count_sam_records,
    json::CommandResponse,
};

#[derive(Debug)]
pub struct EnumerateRequest {
    pub input: PathBuf,
    pub threads: usize,
}

#[derive(Debug, Serialize)]
pub struct EnumeratePayload {
    pub detected_format: DetectedFormat,
    pub records: u64,
}

pub fn run(request: EnumerateRequest) -> CommandResponse<EnumeratePayload> {
    let input = request.input.clone();
    match run_impl(&request) {
        Ok(payload) => CommandResponse::success("enumerate", Some(input.as_path()), payload),
        Err(error) => CommandResponse::failure("enumerate", Some(input.as_path()), error),
    }
}

fn run_impl(request: &EnumerateRequest) -> Result<EnumeratePayload, AppError> {
    let probe = probe_path(&request.input)?;
    let records = match probe.detected_format {
        DetectedFormat::Bam => count_bam_records_pipelined(&request.input)?,
        DetectedFormat::Sam => count_sam_records(&request.input)?,
        DetectedFormat::Fastq => count_fastq_records(&request.input)?,
        DetectedFormat::FastqGz => count_fastq_gz_records(&request.input, request.threads)?,
        DetectedFormat::Fasta => count_fasta_records(&request.input)?,
        DetectedFormat::Unknown => {
            return Err(AppError::UnknownFormat {
                path: request.input.clone(),
            });
        }
        other => {
            return Err(AppError::UnsupportedFormat {
                path: request.input.clone(),
                format: format!(
                    "Enumerate currently supports BAM, SAM, FASTQ, FASTQ.GZ, and FASTA inputs; detected {other}."
                ),
            });
        }
    };

    Ok(EnumeratePayload {
        detected_format: probe.detected_format,
        records,
    })
}

const IO_CHUNK_BYTES: usize = 1024 * 1024;
const DECOMPRESSED_CHUNK_BYTES: usize = 1024 * 1024;
const PIPELINE_DEPTH: usize = 8;

enum PipelineChunk {
    Data(Vec<u8>),
}

struct ChannelRead {
    rx: Receiver<PipelineChunk>,
    current: Vec<u8>,
    cursor: usize,
    eof: bool,
}

impl ChannelRead {
    fn new(rx: Receiver<PipelineChunk>) -> Self {
        Self {
            rx,
            current: Vec::new(),
            cursor: 0,
            eof: false,
        }
    }
}

impl Read for ChannelRead {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.cursor < self.current.len() {
                let available = self.current.len() - self.cursor;
                let count = available.min(buffer.len());
                buffer[..count].copy_from_slice(&self.current[self.cursor..self.cursor + count]);
                self.cursor += count;
                if self.cursor == self.current.len() {
                    self.current.clear();
                    self.cursor = 0;
                }
                return Ok(count);
            }

            if self.eof {
                return Ok(0);
            }

            match self.rx.recv() {
                Ok(PipelineChunk::Data(chunk)) => {
                    self.current = chunk;
                    self.cursor = 0;
                }
                Err(_) => {
                    self.eof = true;
                    return Ok(0);
                }
            }
        }
    }
}

fn count_fastq_gz_records(path: &Path, _threads: usize) -> Result<u64, AppError> {
    Ok(ensure_fastq_gzi(path)?.total_records)
}

fn count_bam_records_pipelined(path: &Path) -> Result<u64, AppError> {
    let (compressed_tx, compressed_rx) = mpsc::sync_channel::<PipelineChunk>(PIPELINE_DEPTH);
    let (decompressed_tx, decompressed_rx) = mpsc::sync_channel::<PipelineChunk>(PIPELINE_DEPTH);
    let path_buf = path.to_path_buf();

    let io_path = path_buf.clone();
    let io_handle = thread::spawn(move || io_thread(&io_path, compressed_tx));

    let decode_path = path_buf.clone();
    let decode_handle =
        thread::spawn(move || decompress_thread(&decode_path, compressed_rx, decompressed_tx));

    let parse_path = path_buf.clone();
    let parse_handle = thread::spawn(move || count_bam_from_chunks(&parse_path, decompressed_rx));

    join_pipeline(io_handle, decode_handle, parse_handle)
}

fn join_pipeline(
    io_handle: thread::JoinHandle<Result<(), AppError>>,
    decode_handle: thread::JoinHandle<Result<(), AppError>>,
    parse_handle: thread::JoinHandle<Result<u64, AppError>>,
) -> Result<u64, AppError> {
    let io_result = io_handle.join().map_err(|_| AppError::Internal {
        message: "enumerate I/O thread panicked".to_string(),
    })?;
    let decode_result = decode_handle.join().map_err(|_| AppError::Internal {
        message: "enumerate decompression thread panicked".to_string(),
    })?;
    let parse_result = parse_handle.join().map_err(|_| AppError::Internal {
        message: "enumerate parser thread panicked".to_string(),
    })?;

    io_result?;
    decode_result?;
    parse_result
}

fn io_thread(path: &Path, tx: SyncSender<PipelineChunk>) -> Result<(), AppError> {
    let mut file = File::open(path).map_err(|error| AppError::from_io(path, error))?;

    loop {
        let mut chunk = vec![0_u8; IO_CHUNK_BYTES];
        let bytes_read = file
            .read(&mut chunk)
            .map_err(|error| AppError::from_io(path, error))?;
        if bytes_read == 0 {
            break;
        }
        chunk.truncate(bytes_read);
        if tx.send(PipelineChunk::Data(chunk)).is_err() {
            break;
        }
    }

    Ok(())
}

fn decompress_thread(
    path: &Path,
    compressed_rx: Receiver<PipelineChunk>,
    decompressed_tx: SyncSender<PipelineChunk>,
) -> Result<(), AppError> {
    let channel_reader = ChannelRead::new(compressed_rx);
    let mut decoder = flate2::read::MultiGzDecoder::new(channel_reader);

    loop {
        let mut chunk = vec![0_u8; DECOMPRESSED_CHUNK_BYTES];
        let bytes_read = decoder.read(&mut chunk).map_err(|error| AppError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        if bytes_read == 0 {
            break;
        }
        chunk.truncate(bytes_read);
        if decompressed_tx.send(PipelineChunk::Data(chunk)).is_err() {
            break;
        }
    }

    Ok(())
}

fn count_bam_from_chunks(path: &Path, rx: Receiver<PipelineChunk>) -> Result<u64, AppError> {
    let mut parser = BamCountParser::new(path.to_path_buf());

    while let Ok(chunk) = rx.recv() {
        match chunk {
            PipelineChunk::Data(chunk) => parser.push(&chunk)?,
        }
    }

    parser.finish()
}

enum BamParseState {
    Magic,
    HeaderTextLen,
    HeaderText(usize),
    ReferenceCount,
    ReferenceNameLen,
    ReferenceName(usize),
    ReferenceLen,
    RecordBlockSize,
    RecordBody(usize),
}

struct BamCountParser {
    path: PathBuf,
    buffer: Vec<u8>,
    cursor: usize,
    state: BamParseState,
    remaining_refs: usize,
    records: u64,
}

impl BamCountParser {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            buffer: Vec::new(),
            cursor: 0,
            state: BamParseState::Magic,
            remaining_refs: 0,
            records: 0,
        }
    }

    fn push(&mut self, chunk: &[u8]) -> Result<(), AppError> {
        self.buffer.extend_from_slice(chunk);
        self.parse_available()
    }

    fn finish(mut self) -> Result<u64, AppError> {
        self.parse_available()?;
        if self.buffer.len() != self.cursor {
            return Err(AppError::TruncatedFile {
                path: self.path,
                detail: "BAM stream ended before a complete header or record was available."
                    .to_string(),
            });
        }
        match self.state {
            BamParseState::RecordBlockSize => Ok(self.records),
            _ => Err(AppError::TruncatedFile {
                path: self.path,
                detail: "BAM stream ended before a complete header or record was available."
                    .to_string(),
            }),
        }
    }

    fn parse_available(&mut self) -> Result<(), AppError> {
        loop {
            let progressed = match self.state {
                BamParseState::Magic => {
                    if let Some(bytes) = self.take(4) {
                        if bytes != b"BAM\x01" {
                            return Err(AppError::InvalidHeader {
                                path: self.path.clone(),
                                detail: "Missing BAM magic in decompressed stream.".to_string(),
                            });
                        }
                        self.state = BamParseState::HeaderTextLen;
                        true
                    } else {
                        false
                    }
                }
                BamParseState::HeaderTextLen => {
                    if let Some(value) = self.read_i32()? {
                        if value < 0 {
                            return Err(AppError::InvalidHeader {
                                path: self.path.clone(),
                                detail: "BAM header text length was negative.".to_string(),
                            });
                        }
                        self.state = BamParseState::HeaderText(value as usize);
                        true
                    } else {
                        false
                    }
                }
                BamParseState::HeaderText(length) => {
                    if self.skip(length) {
                        self.state = BamParseState::ReferenceCount;
                        true
                    } else {
                        false
                    }
                }
                BamParseState::ReferenceCount => {
                    if let Some(value) = self.read_i32()? {
                        if value < 0 {
                            return Err(AppError::InvalidHeader {
                                path: self.path.clone(),
                                detail: "BAM reference count was negative.".to_string(),
                            });
                        }
                        self.remaining_refs = value as usize;
                        self.state = if self.remaining_refs == 0 {
                            BamParseState::RecordBlockSize
                        } else {
                            BamParseState::ReferenceNameLen
                        };
                        true
                    } else {
                        false
                    }
                }
                BamParseState::ReferenceNameLen => {
                    if let Some(value) = self.read_i32()? {
                        if value <= 0 {
                            return Err(AppError::InvalidHeader {
                                path: self.path.clone(),
                                detail: "BAM reference name length was not positive.".to_string(),
                            });
                        }
                        self.state = BamParseState::ReferenceName(value as usize);
                        true
                    } else {
                        false
                    }
                }
                BamParseState::ReferenceName(length) => {
                    if let Some(name) = self.take(length) {
                        if !name.ends_with(&[0]) {
                            return Err(AppError::InvalidHeader {
                                path: self.path.clone(),
                                detail: "BAM reference name was not NUL-terminated.".to_string(),
                            });
                        }
                        self.state = BamParseState::ReferenceLen;
                        true
                    } else {
                        false
                    }
                }
                BamParseState::ReferenceLen => {
                    if let Some(value) = self.read_i32()? {
                        if value < 0 {
                            return Err(AppError::InvalidHeader {
                                path: self.path.clone(),
                                detail: "BAM reference length was negative.".to_string(),
                            });
                        }
                        self.remaining_refs -= 1;
                        self.state = if self.remaining_refs == 0 {
                            BamParseState::RecordBlockSize
                        } else {
                            BamParseState::ReferenceNameLen
                        };
                        true
                    } else {
                        false
                    }
                }
                BamParseState::RecordBlockSize => {
                    let remaining = self.buffer.len().saturating_sub(self.cursor);
                    if remaining == 0 {
                        self.compact();
                        return Ok(());
                    }
                    if let Some(value) = self.read_i32()? {
                        if value < 32 {
                            return Err(AppError::InvalidRecord {
                                path: self.path.clone(),
                                detail: format!(
                                    "BAM record block size {value} is smaller than the 32-byte core alignment section."
                                ),
                            });
                        }
                        self.state = BamParseState::RecordBody(value as usize);
                        true
                    } else {
                        false
                    }
                }
                BamParseState::RecordBody(length) => {
                    if self.skip(length) {
                        self.records += 1;
                        self.state = BamParseState::RecordBlockSize;
                        true
                    } else {
                        false
                    }
                }
            };

            if !progressed {
                self.compact();
                return Ok(());
            }

            if self.cursor > DECOMPRESSED_CHUNK_BYTES {
                self.compact();
            }
        }
    }

    fn take(&mut self, len: usize) -> Option<&[u8]> {
        if self.buffer.len().saturating_sub(self.cursor) < len {
            return None;
        }
        let start = self.cursor;
        self.cursor += len;
        Some(&self.buffer[start..start + len])
    }

    fn skip(&mut self, len: usize) -> bool {
        if self.buffer.len().saturating_sub(self.cursor) < len {
            return false;
        }
        self.cursor += len;
        true
    }

    fn read_i32(&mut self) -> Result<Option<i32>, AppError> {
        let Some(bytes) = self.take(4) else {
            return Ok(None);
        };
        Ok(Some(i32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ])))
    }

    fn compact(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.buffer.drain(..self.cursor);
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, fs::File, io::Write};

    use flate2::{Compression, write::GzEncoder};

    use super::{EnumerateRequest, run};

    #[test]
    fn enumerates_fastq_records() {
        let path = std::env::temp_dir().join(format!(
            "bamana-enumerate-fastq-{}.fastq",
            std::process::id()
        ));
        fs::write(&path, "@read1\nACGT\n+\n!!!!\n@read2\nTTAA\n+\n####\n")
            .expect("fastq should write");

        let response = run(EnumerateRequest {
            input: path.clone(),
            threads: 0,
        });
        fs::remove_file(path).expect("fixture should be removable");

        assert!(response.ok);
        let data = response.data.expect("enumerate data should exist");
        assert_eq!(data.records, 2);
        assert_eq!(data.detected_format.to_string(), "FASTQ");
    }

    #[test]
    fn enumerates_bam_records() {
        let bytes = crate::bgzf::test_support::build_bam_file_with_header_and_records(
            "@SQ\tSN:chr1\tLN:10\n",
            &[("chr1", 10)],
            &[
                crate::bgzf::test_support::build_light_record(0, 0, "read1", 0),
                crate::bgzf::test_support::build_light_record(0, 4, "read2", 0),
            ],
        );
        let path = crate::bgzf::test_support::write_temp_file("enumerate-bam", "bam", &bytes);

        let response = run(EnumerateRequest {
            input: path.clone(),
            threads: 0,
        });
        fs::remove_file(path).expect("fixture should be removable");

        assert!(response.ok);
        let data = response.data.expect("enumerate data should exist");
        assert_eq!(data.records, 2);
        assert_eq!(data.detected_format.to_string(), "BAM");
    }

    #[test]
    fn enumerates_fastq_gz_records() {
        let path = std::env::temp_dir().join(format!(
            "bamana-enumerate-fastq-gz-{}.fastq.gz",
            std::process::id()
        ));
        let index = crate::fastq::gzi::fastq_gzi_output_path(&path);
        let file = File::create(&path).expect("gzip fixture should open");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(b"@read1\nACGT\n+\n!!!!\n@read2\nTTAA\n+\n####\n")
            .expect("gzip fixture should write");
        encoder.finish().expect("gzip fixture should finish");

        let response = run(EnumerateRequest {
            input: path.clone(),
            threads: 0,
        });

        assert!(response.ok);
        let data = response.data.expect("enumerate data should exist");
        assert_eq!(data.records, 2);
        assert_eq!(data.detected_format.to_string(), "FASTQ.GZ");
        assert!(index.is_file());

        let second_response = run(EnumerateRequest {
            input: path.clone(),
            threads: 0,
        });
        fs::remove_file(path).expect("fixture should be removable");
        fs::remove_file(index).expect("index should be removable");

        assert!(second_response.ok);
        let second_data = second_response
            .data
            .expect("second enumerate data should exist");
        assert_eq!(second_data.records, 2);
    }
}
