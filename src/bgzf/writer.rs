use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use rayon::{ThreadPool, prelude::*};

use crate::{
    bgzf::block::{
        BGZF_EOF_MARKER, BGZF_TARGET_UNCOMPRESSED_BLOCK, build_bgzf_member_fitting_with_level,
    },
    error::AppError,
};

pub struct BgzfWriter {
    path: PathBuf,
    writer: BufWriter<File>,
    buffer: Vec<u8>,
    compression_threads: usize,
    compression_pool: Option<ThreadPool>,
    compression_level: u32,
}

impl BgzfWriter {
    pub fn create(path: &Path) -> Result<Self, AppError> {
        Self::create_with_threads(path, 1)
    }

    pub fn create_with_threads(path: &Path, threads: usize) -> Result<Self, AppError> {
        Self::create_with_threads_and_level(path, threads, 6)
    }

    pub fn create_with_threads_and_level(
        path: &Path,
        threads: usize,
        compression_level: u32,
    ) -> Result<Self, AppError> {
        if compression_level > 9 {
            return Err(AppError::WriteError {
                path: path.to_path_buf(),
                message: "BGZF compression level must be between 0 and 9.".to_string(),
            });
        }
        let compression_threads = threads.max(1);
        let compression_pool = if compression_threads == 1 {
            None
        } else {
            Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(compression_threads)
                    .build()
                    .map_err(|error| AppError::WriteError {
                        path: path.to_path_buf(),
                        message: format!("Could not create BGZF compression pool: {error}"),
                    })?,
            )
        };
        let file = File::create(path).map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        Ok(Self {
            path: path.to_path_buf(),
            writer: BufWriter::new(file),
            buffer: Vec::with_capacity(BGZF_TARGET_UNCOMPRESSED_BLOCK * (compression_threads + 1)),
            compression_threads,
            compression_pool,
            compression_level,
        })
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), AppError> {
        self.buffer.extend_from_slice(bytes);
        let batch_size = BGZF_TARGET_UNCOMPRESSED_BLOCK * self.compression_threads;
        while self.buffer.len() >= batch_size {
            self.flush_full_block_batch(self.compression_threads)?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), AppError> {
        while self.buffer.len() >= BGZF_TARGET_UNCOMPRESSED_BLOCK {
            let full_blocks = self.buffer.len() / BGZF_TARGET_UNCOMPRESSED_BLOCK;
            self.flush_full_block_batch(full_blocks.min(self.compression_threads))?;
        }
        while !self.buffer.is_empty() {
            self.flush_next_block(true)?;
        }

        self.writer
            .write_all(&BGZF_EOF_MARKER)
            .map_err(|error| AppError::WriteError {
                path: self.path.clone(),
                message: error.to_string(),
            })?;
        self.writer.flush().map_err(|error| AppError::WriteError {
            path: self.path.clone(),
            message: error.to_string(),
        })?;
        Ok(())
    }

    fn flush_full_block_batch(&mut self, block_count: usize) -> Result<(), AppError> {
        if block_count <= 1 || self.compression_pool.is_none() {
            return self.flush_next_block(false);
        }

        let batch_len = BGZF_TARGET_UNCOMPRESSED_BLOCK * block_count;
        let chunks = self.buffer[..batch_len].chunks_exact(BGZF_TARGET_UNCOMPRESSED_BLOCK);
        let results = self
            .compression_pool
            .as_ref()
            .expect("parallel batch requires a compression pool")
            .install(|| {
                chunks
                    .collect::<Vec<_>>()
                    .into_par_iter()
                    .map(|chunk| {
                        build_bgzf_member_fitting_with_level(chunk, self.compression_level)
                    })
                    .collect::<Vec<_>>()
            });

        let members = results
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|message| AppError::WriteError {
                path: self.path.clone(),
                message,
            })?;
        if members
            .iter()
            .any(|(_, consumed)| *consumed != BGZF_TARGET_UNCOMPRESSED_BLOCK)
        {
            return self.flush_next_block(false);
        }
        for (member, _) in members {
            self.writer
                .write_all(&member)
                .map_err(|error| AppError::WriteError {
                    path: self.path.clone(),
                    message: error.to_string(),
                })?;
        }
        self.buffer.drain(..batch_len);
        Ok(())
    }

    fn flush_next_block(&mut self, allow_small_block: bool) -> Result<(), AppError> {
        let max_candidate = if allow_small_block {
            self.buffer.len()
        } else {
            self.buffer.len().min(BGZF_TARGET_UNCOMPRESSED_BLOCK)
        };
        let (member, consumed) = build_bgzf_member_fitting_with_level(
            &self.buffer[..max_candidate],
            self.compression_level,
        )
        .map_err(|message| AppError::WriteError {
            path: self.path.clone(),
            message,
        })?;
        self.writer
            .write_all(&member)
            .map_err(|error| AppError::WriteError {
                path: self.path.clone(),
                message: error.to_string(),
            })?;
        self.buffer.drain(..consumed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::bgzf::{
        BGZF_EOF_MARKER,
        block::BGZF_TARGET_UNCOMPRESSED_BLOCK,
        reader::{has_bgzf_eof, read_bgzf_payloads},
    };

    use super::BgzfWriter;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bamana-bgzf-writer-{name}-{}-{nonce}.bam",
            std::process::id()
        ))
    }

    fn write_payload(path: &std::path::Path, payload: &[u8]) {
        let mut writer = BgzfWriter::create(path).expect("writer should create");
        writer.write_all(payload).expect("payload should write");
        writer.finish().expect("writer should finish");
    }

    fn read_file(path: &std::path::Path) -> Vec<u8> {
        fs::read(path).expect("writer output should be readable")
    }

    fn repeated_payload(len: usize) -> Vec<u8> {
        (0..len).map(|index| (index % 251) as u8).collect()
    }

    #[test]
    fn writes_empty_stream_as_canonical_eof_marker() {
        let path = temp_path("empty");

        let writer = BgzfWriter::create(&path).expect("writer should create");
        writer.finish().expect("writer should finish");

        let bytes = read_file(&path);
        let payloads = read_bgzf_payloads(&path).expect("native reader should accept output");
        let eof_present = has_bgzf_eof(&path).expect("eof check should succeed");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(bytes, BGZF_EOF_MARKER);
        assert!(payloads.is_empty());
        assert!(eof_present);
    }

    #[test]
    fn round_trips_single_payload_through_native_reader() {
        let path = temp_path("single-payload");
        let payload = b"BAM\x01writer round trip payload";

        write_payload(&path, payload);

        let payloads = read_bgzf_payloads(&path).expect("native reader should accept output");

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(payloads, vec![payload.to_vec()]);
    }

    #[test]
    fn writes_payloads_that_require_multiple_bgzf_blocks() {
        let path = temp_path("multi-block");
        let payload = repeated_payload(BGZF_TARGET_UNCOMPRESSED_BLOCK * 2 + 777);

        write_payload(&path, &payload);

        let payloads = read_bgzf_payloads(&path).expect("native reader should accept output");
        let observed: Vec<u8> = payloads.into_iter().flatten().collect();

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(observed, payload);
    }

    #[test]
    fn emits_deterministic_eof_marker_once() {
        let path = temp_path("deterministic-eof");
        let payload = b"payload";

        write_payload(&path, payload);

        let bytes = read_file(&path);
        let eof_count = bytes
            .windows(BGZF_EOF_MARKER.len())
            .filter(|window| *window == BGZF_EOF_MARKER)
            .count();

        fs::remove_file(path).expect("fixture should be removed");
        assert!(bytes.ends_with(&BGZF_EOF_MARKER));
        assert_eq!(eof_count, 1);
    }

    #[test]
    fn handles_payload_at_target_block_boundary() {
        let path = temp_path("target-boundary");
        let payload = repeated_payload(BGZF_TARGET_UNCOMPRESSED_BLOCK);

        write_payload(&path, &payload);

        let payloads = read_bgzf_payloads(&path).expect("native reader should accept output");
        let observed: Vec<u8> = payloads.into_iter().flatten().collect();

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(observed, payload);
    }

    #[test]
    fn handles_payload_just_over_target_block_boundary() {
        let path = temp_path("over-target-boundary");
        let payload = repeated_payload(BGZF_TARGET_UNCOMPRESSED_BLOCK + 1);

        write_payload(&path, &payload);

        let payloads = read_bgzf_payloads(&path).expect("native reader should accept output");
        assert!(
            payloads.len() >= 2,
            "payload just over the target block should span multiple members"
        );
        let observed: Vec<u8> = payloads.into_iter().flatten().collect();

        fs::remove_file(path).expect("fixture should be removed");
        assert_eq!(observed, payload);
    }

    #[test]
    fn parallel_compression_preserves_payload_and_deterministic_bytes() {
        let serial_path = temp_path("serial-determinism");
        let parallel_path = temp_path("parallel-determinism");
        let payload = repeated_payload(BGZF_TARGET_UNCOMPRESSED_BLOCK * 7 + 777);

        write_payload(&serial_path, &payload);
        let mut parallel =
            BgzfWriter::create_with_threads(&parallel_path, 4).expect("writer should create");
        for chunk in payload.chunks(997) {
            parallel.write_all(chunk).expect("payload should write");
        }
        parallel.finish().expect("writer should finish");

        let parallel_payload: Vec<u8> = read_bgzf_payloads(&parallel_path)
            .expect("native reader should accept parallel output")
            .into_iter()
            .flatten()
            .collect();
        let serial_bytes = read_file(&serial_path);
        let parallel_bytes = read_file(&parallel_path);

        fs::remove_file(serial_path).expect("fixture should be removed");
        fs::remove_file(parallel_path).expect("fixture should be removed");
        assert_eq!(parallel_payload, payload);
        assert_eq!(parallel_bytes, serial_bytes);
    }

    #[test]
    fn explicit_fast_compression_is_deterministic_and_round_trips() {
        let first_path = temp_path("fast-level-first");
        let second_path = temp_path("fast-level-second");
        let payload = repeated_payload(BGZF_TARGET_UNCOMPRESSED_BLOCK * 3 + 777);
        for path in [&first_path, &second_path] {
            let mut writer = BgzfWriter::create_with_threads_and_level(path, 4, 1)
                .expect("fast writer should create");
            writer.write_all(&payload).expect("payload should write");
            writer.finish().expect("writer should finish");
        }
        assert_eq!(read_file(&first_path), read_file(&second_path));
        assert_eq!(
            read_bgzf_payloads(&first_path)
                .expect("fast output should read")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            payload
        );
        fs::remove_file(first_path).expect("fixture should be removed");
        fs::remove_file(second_path).expect("fixture should be removed");
    }
}
