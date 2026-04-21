use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
};

use flate2::read::MultiGzDecoder;

use crate::error::AppError;

const FASTQ_GZI_MAGIC: &[u8; 8] = b"FQGZI\0\0\x01";
pub const DEFAULT_INTERVAL_PERCENT: f64 = 1.0;
pub const DEFAULT_INTERVAL_BASIS_POINTS: u32 = 100;
const TOTAL_BASIS_POINTS: u64 = 10_000;
const IO_CHUNK_BYTES: usize = 1024 * 1024;
const DECOMPRESSED_CHUNK_BYTES: usize = 1024 * 1024;
const PIPELINE_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastqGziCheckpoint {
    pub compressed_offset: u64,
    pub uncompressed_offset: u64,
    pub records: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastqGziIndexSummary {
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub total_records: u64,
    pub interval_basis_points: u32,
    pub checkpoints: Vec<FastqGziCheckpoint>,
}

pub fn fastq_gzi_output_path(path: &Path) -> PathBuf {
    let mut output = path.to_path_buf();
    output.set_extension("gzi");
    output
}

pub fn build_fastq_gzi(path: &Path, out: &Path) -> Result<FastqGziIndexSummary, AppError> {
    let summary = sample_fastq_gzi(path, DEFAULT_INTERVAL_BASIS_POINTS)?;
    write_fastq_gzi(out, &summary)?;
    Ok(summary)
}

pub fn ensure_fastq_gzi(path: &Path) -> Result<FastqGziIndexSummary, AppError> {
    let output = fastq_gzi_output_path(path);
    if output.is_file() {
        read_fastq_gzi(&output)
    } else {
        build_fastq_gzi(path, &output)
    }
}

pub fn sample_fastq_gzi(
    path: &Path,
    interval_basis_points: u32,
) -> Result<FastqGziIndexSummary, AppError> {
    if interval_basis_points == 0 || interval_basis_points > TOTAL_BASIS_POINTS as u32 {
        return Err(AppError::UnsupportedIndex {
            path: path.to_path_buf(),
            detail: format!(
                "FASTQ.GZI interval basis points must be between 1 and {TOTAL_BASIS_POINTS}."
            ),
        });
    }

    let compressed_size = fs::metadata(path)
        .map_err(|error| AppError::from_io(path, error))?
        .len();
    let compressed_counter = Arc::new(AtomicU64::new(0));
    let (compressed_tx, compressed_rx) = mpsc::sync_channel::<PipelineChunk>(PIPELINE_DEPTH);
    let (decompressed_tx, decompressed_rx) = mpsc::sync_channel::<PipelineChunk>(PIPELINE_DEPTH);
    let io_path = path.to_path_buf();
    let decode_path = path.to_path_buf();
    let positional_path = path.to_path_buf();

    let io_handle = thread::spawn(move || io_thread(&io_path, compressed_tx));
    let decode_counter = Arc::clone(&compressed_counter);
    let decode_handle = thread::spawn(move || {
        decompress_thread(&decode_path, compressed_rx, decompressed_tx, decode_counter)
    });
    let positional_counter = Arc::clone(&compressed_counter);
    let positional_handle = thread::spawn(move || {
        positional_thread(
            &positional_path,
            decompressed_rx,
            compressed_size,
            interval_basis_points,
            positional_counter,
        )
    });

    let io_result = io_handle.join().map_err(|_| AppError::Internal {
        message: "FASTQ.GZI I/O thread panicked".to_string(),
    })?;
    let decode_result = decode_handle.join().map_err(|_| AppError::Internal {
        message: "FASTQ.GZI decompression thread panicked".to_string(),
    })?;
    let positional_result = positional_handle.join().map_err(|_| AppError::Internal {
        message: "FASTQ.GZI positional thread panicked".to_string(),
    })?;

    io_result?;
    decode_result?;
    positional_result
}

pub fn read_fastq_gzi(path: &Path) -> Result<FastqGziIndexSummary, AppError> {
    let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
    let mut reader = BufReader::new(file);

    let mut magic = [0_u8; 8];
    reader
        .read_exact(&mut magic)
        .map_err(|error| AppError::from_io(path, error))?;
    if &magic != FASTQ_GZI_MAGIC {
        return Err(AppError::InvalidIndex {
            path: path.to_path_buf(),
            detail: "FASTQ.GZI magic did not match the expected Bamana sidecar header.".to_string(),
        });
    }

    let interval_basis_points = read_u32(&mut reader, path)?;
    let compressed_size = read_u64(&mut reader, path)?;
    let uncompressed_size = read_u64(&mut reader, path)?;
    let total_records = read_u64(&mut reader, path)?;
    let checkpoint_count = read_u64(&mut reader, path)?;

    let mut checkpoints = Vec::with_capacity(checkpoint_count as usize);
    for _ in 0..checkpoint_count {
        checkpoints.push(FastqGziCheckpoint {
            compressed_offset: read_u64(&mut reader, path)?,
            uncompressed_offset: read_u64(&mut reader, path)?,
            records: read_u64(&mut reader, path)?,
        });
    }

    Ok(FastqGziIndexSummary {
        compressed_size,
        uncompressed_size,
        total_records,
        interval_basis_points,
        checkpoints,
    })
}

fn advance_thresholds(
    compressed_size: u64,
    compressed_offset: u64,
    interval_basis_points: u64,
    next_threshold_basis_points: &mut u64,
) -> bool {
    let mut crossed = false;
    while *next_threshold_basis_points < TOTAL_BASIS_POINTS
        && compressed_offset.saturating_mul(TOTAL_BASIS_POINTS)
            >= compressed_size.saturating_mul(*next_threshold_basis_points)
    {
        crossed = true;
        *next_threshold_basis_points += interval_basis_points;
    }
    crossed
}

fn write_fastq_gzi(path: &Path, summary: &FastqGziIndexSummary) -> Result<(), AppError> {
    let file = File::create(path).map_err(|error| AppError::WriteError {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut writer = BufWriter::new(file);

    writer
        .write_all(FASTQ_GZI_MAGIC)
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    writer
        .write_all(&summary.interval_basis_points.to_le_bytes())
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    writer
        .write_all(&summary.compressed_size.to_le_bytes())
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    writer
        .write_all(&summary.uncompressed_size.to_le_bytes())
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    writer
        .write_all(&summary.total_records.to_le_bytes())
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    writer
        .write_all(&(summary.checkpoints.len() as u64).to_le_bytes())
        .map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    for checkpoint in &summary.checkpoints {
        writer
            .write_all(&checkpoint.compressed_offset.to_le_bytes())
            .and_then(|()| writer.write_all(&checkpoint.uncompressed_offset.to_le_bytes()))
            .and_then(|()| writer.write_all(&checkpoint.records.to_le_bytes()))
            .map_err(|error| AppError::WriteError {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
    }

    writer.flush().map_err(|error| AppError::WriteError {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    Ok(())
}

enum PipelineChunk {
    Data(Vec<u8>),
}

struct ChannelRead {
    rx: Receiver<PipelineChunk>,
    bytes_read: Arc<AtomicU64>,
    current: Vec<u8>,
    cursor: usize,
    eof: bool,
}

impl ChannelRead {
    fn new(rx: Receiver<PipelineChunk>, bytes_read: Arc<AtomicU64>) -> Self {
        Self {
            rx,
            bytes_read,
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
                    self.bytes_read
                        .fetch_add(chunk.len() as u64, Ordering::Relaxed);
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
    compressed_counter: Arc<AtomicU64>,
) -> Result<(), AppError> {
    let channel_reader = ChannelRead::new(compressed_rx, compressed_counter);
    let mut decoder = MultiGzDecoder::new(channel_reader);

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

fn positional_thread(
    path: &Path,
    decompressed_rx: Receiver<PipelineChunk>,
    compressed_size: u64,
    interval_basis_points: u32,
    compressed_counter: Arc<AtomicU64>,
) -> Result<FastqGziIndexSummary, AppError> {
    let mut checkpoints = vec![FastqGziCheckpoint {
        compressed_offset: 0,
        uncompressed_offset: 0,
        records: 0,
    }];
    let mut next_threshold_basis_points = interval_basis_points as u64;
    let mut uncompressed_offset = 0_u64;
    let mut total_records = 0_u64;
    let mut pending = Vec::new();
    let mut record_bytes = 0_u64;
    let mut state = 0_u8;
    let mut sequence_len = 0_usize;

    while let Ok(PipelineChunk::Data(chunk)) = decompressed_rx.recv() {
        pending.extend_from_slice(&chunk);
        let mut start = 0_usize;

        for index in 0..pending.len() {
            if pending[index] != b'\n' {
                continue;
            }

            let raw_line_len = (index + 1 - start) as u64;
            let mut line = &pending[start..index];
            if line.ends_with(b"\r") {
                line = &line[..line.len() - 1];
            }

            match state {
                0 => {
                    if !line.starts_with(b"@") {
                        return Err(AppError::InvalidFastq {
                            path: path.to_path_buf(),
                            detail: "FASTQ record header line did not start with '@'.".to_string(),
                        });
                    }
                }
                1 => {
                    sequence_len = line.len();
                }
                2 => {
                    if !line.starts_with(b"+") {
                        return Err(AppError::InvalidFastq {
                            path: path.to_path_buf(),
                            detail: "FASTQ record plus line did not start with '+'.".to_string(),
                        });
                    }
                }
                3 => {
                    if line.len() != sequence_len {
                        return Err(AppError::InvalidFastq {
                            path: path.to_path_buf(),
                            detail: format!(
                                "FASTQ sequence and quality lengths differed ({} vs {}).",
                                sequence_len,
                                line.len()
                            ),
                        });
                    }
                }
                _ => unreachable!(),
            }

            record_bytes += raw_line_len;
            if state == 3 {
                total_records += 1;
                uncompressed_offset += record_bytes;
                record_bytes = 0;

                let compressed_offset = compressed_counter.load(Ordering::Relaxed);
                let crossed = advance_thresholds(
                    compressed_size,
                    compressed_offset,
                    interval_basis_points as u64,
                    &mut next_threshold_basis_points,
                );
                if crossed
                    && checkpoints.last().is_none_or(|checkpoint| {
                        checkpoint.compressed_offset != compressed_offset
                            || checkpoint.uncompressed_offset != uncompressed_offset
                            || checkpoint.records != total_records
                    })
                {
                    checkpoints.push(FastqGziCheckpoint {
                        compressed_offset,
                        uncompressed_offset,
                        records: total_records,
                    });
                }
            }

            state = (state + 1) % 4;
            start = index + 1;
        }

        if start > 0 {
            pending.drain(..start);
        }
    }

    if !pending.is_empty() || state != 0 {
        return Err(AppError::InvalidFastq {
            path: path.to_path_buf(),
            detail: "FASTQ ended before a complete record was available.".to_string(),
        });
    }

    if checkpoints.last().is_none_or(|checkpoint| {
        checkpoint.compressed_offset != compressed_size
            || checkpoint.uncompressed_offset != uncompressed_offset
            || checkpoint.records != total_records
    }) {
        checkpoints.push(FastqGziCheckpoint {
            compressed_offset: compressed_size,
            uncompressed_offset,
            records: total_records,
        });
    }

    Ok(FastqGziIndexSummary {
        compressed_size,
        uncompressed_size: uncompressed_offset,
        total_records,
        interval_basis_points,
        checkpoints,
    })
}

fn read_u32(reader: &mut impl Read, path: &Path) -> Result<u32, AppError> {
    let mut bytes = [0_u8; 4];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| AppError::from_io(path, error))?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read, path: &Path) -> Result<u64, AppError> {
    let mut bytes = [0_u8; 8];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| AppError::from_io(path, error))?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
    };

    use flate2::{Compression, write::GzEncoder};

    use super::{
        DEFAULT_INTERVAL_BASIS_POINTS, DEFAULT_INTERVAL_PERCENT, FASTQ_GZI_MAGIC, build_fastq_gzi,
        sample_fastq_gzi,
    };

    #[test]
    fn samples_fastq_gzi_boundaries() {
        let input = std::env::temp_dir().join(format!(
            "bamana-fastq-gzi-sample-{}.fastq.gz",
            std::process::id()
        ));
        let file = fs::File::create(&input).expect("fixture should create");
        let mut encoder = GzEncoder::new(file, Compression::default());
        for index in 0..20 {
            writeln!(encoder, "@read{index}").expect("header should write");
            writeln!(encoder, "ACGTACGT").expect("sequence should write");
            writeln!(encoder, "+").expect("plus should write");
            writeln!(encoder, "!!!!!!!!").expect("quality should write");
        }
        encoder.finish().expect("gzip should finish");

        let summary = sample_fastq_gzi(&input, 2_500).expect("gzi summary should build");
        fs::remove_file(input).expect("fixture should remove");

        assert_eq!(DEFAULT_INTERVAL_BASIS_POINTS, 100);
        assert_eq!(DEFAULT_INTERVAL_PERCENT, 1.0);
        assert!(summary.checkpoints.len() >= 3);
        assert_eq!(summary.checkpoints.first().unwrap().compressed_offset, 0);
        assert_eq!(summary.checkpoints.first().unwrap().uncompressed_offset, 0);
        assert_eq!(
            summary.checkpoints.last().unwrap().compressed_offset,
            summary.compressed_size
        );
        assert_eq!(
            summary.checkpoints.last().unwrap().uncompressed_offset,
            summary.uncompressed_size
        );
    }

    #[test]
    fn writes_fastq_gzi_binary_index() {
        let input = std::env::temp_dir().join(format!(
            "bamana-fastq-gzi-write-{}.fastq.gz",
            std::process::id()
        ));
        let output = std::env::temp_dir().join(format!(
            "bamana-fastq-gzi-write-{}.fastq.gzi",
            std::process::id()
        ));
        let file = fs::File::create(&input).expect("fixture should create");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(b"@read1\nACGT\n+\n!!!!\n@read2\nTTAA\n+\n####\n")
            .expect("fixture should write");
        encoder.finish().expect("gzip should finish");

        let summary = build_fastq_gzi(&input, &output).expect("gzi index should build");
        let mut bytes = Vec::new();
        fs::File::open(&output)
            .expect("index should exist")
            .read_to_end(&mut bytes)
            .expect("index should read");
        fs::remove_file(input).expect("fixture should remove");
        fs::remove_file(output).expect("index should remove");

        assert!(bytes.starts_with(FASTQ_GZI_MAGIC));
        assert!(summary.checkpoints.len() >= 2);
    }
}
