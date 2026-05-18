use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{
    bgzf::block::{BGZF_EOF_MARKER, BGZF_TARGET_UNCOMPRESSED_BLOCK, build_bgzf_member_fitting},
    error::AppError,
};

pub struct BgzfWriter {
    path: PathBuf,
    writer: BufWriter<File>,
    buffer: Vec<u8>,
}

impl BgzfWriter {
    pub fn create(path: &Path) -> Result<Self, AppError> {
        let file = File::create(path).map_err(|error| AppError::WriteError {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        Ok(Self {
            path: path.to_path_buf(),
            writer: BufWriter::new(file),
            buffer: Vec::with_capacity(BGZF_TARGET_UNCOMPRESSED_BLOCK * 2),
        })
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), AppError> {
        self.buffer.extend_from_slice(bytes);
        while self.buffer.len() >= BGZF_TARGET_UNCOMPRESSED_BLOCK {
            self.flush_next_block(false)?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), AppError> {
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

    fn flush_next_block(&mut self, allow_small_block: bool) -> Result<(), AppError> {
        let max_candidate = if allow_small_block {
            self.buffer.len()
        } else {
            self.buffer.len().min(BGZF_TARGET_UNCOMPRESSED_BLOCK)
        };
        let (member, consumed) =
            build_bgzf_member_fitting(&self.buffer[..max_candidate]).map_err(|message| {
                AppError::WriteError {
                    path: self.path.clone(),
                    message,
                }
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
