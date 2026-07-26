use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
};

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
    compression_pipeline: Option<CompressionPipeline>,
    compression_level: u32,
}

struct CompressionJob {
    sequence: u64,
    payload: Vec<u8>,
}

struct CompressionResult {
    sequence: u64,
    member: Result<(Vec<u8>, usize), String>,
}

struct CompressionPipeline {
    jobs: Option<mpsc::SyncSender<CompressionJob>>,
    results: mpsc::Receiver<CompressionResult>,
    workers: Vec<thread::JoinHandle<()>>,
    pending: BTreeMap<u64, Result<(Vec<u8>, usize), String>>,
    submitted: u64,
    written: u64,
    max_in_flight: u64,
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
        let compression_pipeline = if compression_threads == 1 {
            None
        } else {
            Some(CompressionPipeline::new(
                compression_threads,
                compression_level,
                path,
            )?)
        };
        let file = File::create(path).map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        Ok(Self {
            path: path.to_path_buf(),
            writer: BufWriter::new(file),
            buffer: Vec::with_capacity(BGZF_TARGET_UNCOMPRESSED_BLOCK * 2),
            compression_pipeline,
            compression_level,
        })
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), AppError> {
        self.buffer.extend_from_slice(bytes);
        while self.buffer.len() >= BGZF_TARGET_UNCOMPRESSED_BLOCK {
            self.flush_full_block()?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), AppError> {
        while self.buffer.len() >= BGZF_TARGET_UNCOMPRESSED_BLOCK {
            self.flush_full_block()?;
        }
        if let Some(mut pipeline) = self.compression_pipeline.take() {
            pipeline.finish(&mut self.writer, &self.path)?;
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

    fn flush_full_block(&mut self) -> Result<(), AppError> {
        if self.compression_pipeline.is_none() {
            return self.flush_next_block(false);
        }

        let payload = self
            .buffer
            .drain(..BGZF_TARGET_UNCOMPRESSED_BLOCK)
            .collect();
        self.compression_pipeline
            .as_mut()
            .expect("parallel writer has a compression pipeline")
            .submit(payload, &mut self.writer, &self.path)
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

impl CompressionPipeline {
    fn new(threads: usize, compression_level: u32, path: &Path) -> Result<Self, AppError> {
        let max_in_flight = threads.saturating_mul(2).max(2);
        let (job_tx, job_rx) = mpsc::sync_channel::<CompressionJob>(max_in_flight);
        let (result_tx, result_rx) = mpsc::channel();
        let shared_jobs = Arc::new(Mutex::new(job_rx));
        let mut workers = Vec::with_capacity(threads);
        for worker_index in 0..threads {
            let jobs = Arc::clone(&shared_jobs);
            let results = result_tx.clone();
            let worker = thread::Builder::new()
                .name(format!("bamana-bgzf-{worker_index}"))
                .spawn(move || {
                    loop {
                        let job = {
                            let receiver = match jobs.lock() {
                                Ok(receiver) => receiver,
                                Err(_) => break,
                            };
                            match receiver.recv() {
                                Ok(job) => job,
                                Err(_) => break,
                            }
                        };
                        let member =
                            build_bgzf_member_fitting_with_level(&job.payload, compression_level);
                        if results
                            .send(CompressionResult {
                                sequence: job.sequence,
                                member,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .map_err(|error| AppError::WriteError {
                    path: path.to_path_buf(),
                    message: format!("Could not create BGZF compression worker: {error}"),
                })?;
            workers.push(worker);
        }
        drop(result_tx);
        Ok(Self {
            jobs: Some(job_tx),
            results: result_rx,
            workers,
            pending: BTreeMap::new(),
            submitted: 0,
            written: 0,
            max_in_flight: max_in_flight as u64,
        })
    }

    fn submit(
        &mut self,
        payload: Vec<u8>,
        writer: &mut BufWriter<File>,
        path: &Path,
    ) -> Result<(), AppError> {
        while self.submitted - self.written >= self.max_in_flight {
            self.receive_one(writer, path)?;
        }
        let sequence = self.submitted;
        self.jobs
            .as_ref()
            .expect("active compression pipeline has a sender")
            .send(CompressionJob { sequence, payload })
            .map_err(|_| AppError::WriteError {
                path: path.to_path_buf(),
                message: "BGZF compression workers stopped before accepting all blocks."
                    .to_string(),
            })?;
        self.submitted += 1;
        self.write_ready(writer, path)
    }

    fn receive_one(&mut self, writer: &mut BufWriter<File>, path: &Path) -> Result<(), AppError> {
        let result = self.results.recv().map_err(|_| AppError::WriteError {
            path: path.to_path_buf(),
            message: "BGZF compression workers stopped before returning all blocks.".to_string(),
        })?;
        self.pending.insert(result.sequence, result.member);
        self.write_ready(writer, path)
    }

    fn write_ready(&mut self, writer: &mut BufWriter<File>, path: &Path) -> Result<(), AppError> {
        while let Some(member) = self.pending.remove(&self.written) {
            let (member, consumed) = member.map_err(|message| AppError::WriteError {
                path: path.to_path_buf(),
                message,
            })?;
            if consumed != BGZF_TARGET_UNCOMPRESSED_BLOCK {
                return Err(AppError::WriteError {
                    path: path.to_path_buf(),
                    message: format!(
                        "Parallel BGZF block consumed {consumed} bytes instead of the required {BGZF_TARGET_UNCOMPRESSED_BLOCK}."
                    ),
                });
            }
            writer
                .write_all(&member)
                .map_err(|error| AppError::WriteError {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;
            self.written += 1;
        }
        Ok(())
    }

    fn finish(&mut self, writer: &mut BufWriter<File>, path: &Path) -> Result<(), AppError> {
        self.jobs.take();
        while self.written < self.submitted {
            self.receive_one(writer, path)?;
        }
        for worker in self.workers.drain(..) {
            worker.join().map_err(|_| AppError::WriteError {
                path: path.to_path_buf(),
                message: "BGZF compression worker panicked.".to_string(),
            })?;
        }
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

    fn high_entropy_payload(len: usize) -> Vec<u8> {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
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

    #[test]
    fn pipelined_level_zero_handles_high_entropy_full_blocks() {
        let serial_path = temp_path("level-zero-serial");
        let pipeline_path = temp_path("level-zero-pipeline");
        let payload = high_entropy_payload(BGZF_TARGET_UNCOMPRESSED_BLOCK * 9 + 311);
        let mut serial = BgzfWriter::create_with_threads_and_level(&serial_path, 1, 0)
            .expect("serial writer should create");
        serial
            .write_all(&payload)
            .expect("serial payload should write");
        serial.finish().expect("serial writer should finish");
        let mut pipeline = BgzfWriter::create_with_threads_and_level(&pipeline_path, 6, 0)
            .expect("pipeline writer should create");
        for chunk in payload.chunks(1231) {
            pipeline.write_all(chunk).expect("payload should write");
        }
        pipeline.finish().expect("pipeline writer should finish");
        assert_eq!(read_file(&pipeline_path), read_file(&serial_path));
        assert_eq!(
            read_bgzf_payloads(&pipeline_path)
                .expect("pipeline output should read")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            payload
        );
        fs::remove_file(serial_path).expect("fixture should be removed");
        fs::remove_file(pipeline_path).expect("fixture should be removed");
    }

    #[test]
    fn pipelined_fast_compression_handles_high_entropy_full_blocks() {
        let path = temp_path("level-one-high-entropy");
        let payload = high_entropy_payload(BGZF_TARGET_UNCOMPRESSED_BLOCK * 9 + 311);
        let mut writer = BgzfWriter::create_with_threads_and_level(&path, 6, 1)
            .expect("pipeline writer should create");
        for chunk in payload.chunks(1231) {
            writer.write_all(chunk).expect("payload should write");
        }
        writer.finish().expect("pipeline writer should finish");
        assert_eq!(
            read_bgzf_payloads(&path)
                .expect("pipeline output should read")
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            payload
        );
        fs::remove_file(path).expect("fixture should be removed");
    }
}
