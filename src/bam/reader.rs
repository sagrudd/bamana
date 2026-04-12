use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use flate2::read::MultiGzDecoder;

use crate::error::AppError;

pub struct BamReader {
    path: PathBuf,
    decoder: MultiGzDecoder<BufReader<File>>,
}

impl BamReader {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
        let decoder = MultiGzDecoder::new(BufReader::new(file));

        Ok(Self {
            path: path.to_path_buf(),
            decoder,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read_magic(&mut self) -> Result<[u8; 4], AppError> {
        let mut magic = [0_u8; 4];
        self.read_exact_into(&mut magic)?;
        Ok(magic)
    }

    pub fn read_i32_le(&mut self) -> Result<i32, AppError> {
        let mut bytes = [0_u8; 4];
        self.read_exact_into(&mut bytes)?;
        Ok(i32::from_le_bytes(bytes))
    }

    pub fn read_optional_i32_le(&mut self) -> Result<Option<i32>, AppError> {
        let mut bytes = [0_u8; 4];
        match self.decoder.read(&mut bytes[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(error) => return Err(AppError::from_io(&self.path, error)),
        }

        self.decoder.read_exact(&mut bytes[1..]).map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                AppError::TruncatedFile {
                    path: self.path.clone(),
                    detail: "BAM stream ended while reading the next record block size."
                        .to_string(),
                }
            } else {
                AppError::from_io(&self.path, error)
            }
        })?;

        Ok(Some(i32::from_le_bytes(bytes)))
    }

    pub fn read_u32_le(&mut self) -> Result<u32, AppError> {
        let mut bytes = [0_u8; 4];
        self.read_exact_into(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_exact_vec(&mut self, len: usize) -> Result<Vec<u8>, AppError> {
        let mut buffer = vec![0_u8; len];
        self.read_exact_into(&mut buffer)?;
        Ok(buffer)
    }

    pub fn skip_exact(&mut self, mut len: usize) -> Result<(), AppError> {
        let mut buffer = [0_u8; 8192];
        while len > 0 {
            let chunk = len.min(buffer.len());
            self.read_exact_into(&mut buffer[..chunk])?;
            len -= chunk;
        }
        Ok(())
    }

    fn read_exact_into(&mut self, buffer: &mut [u8]) -> Result<(), AppError> {
        self.decoder.read_exact(buffer).map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                AppError::TruncatedFile {
                    path: self.path.clone(),
                    detail: "BAM stream ended before the expected number of bytes were available."
                        .to_string(),
                }
            } else {
                AppError::from_io(&self.path, error)
            }
        })
    }
}
