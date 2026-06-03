use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use flate2::read::MultiGzDecoder;

use crate::{
    bgzf::{reader::NativeBgzfReader, virtual_offset::VirtualOffset},
    error::AppError,
};

pub struct BamReader {
    path: PathBuf,
    backend: BamReaderBackend,
}

enum BamReaderBackend {
    Gzip(MultiGzDecoder<BufReader<File>>),
    NativeBgzf(NativeBgzfReader),
}

impl BamReader {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Self::open_with_label(path, path)
    }

    pub fn open_with_label(path: &Path, label: &Path) -> Result<Self, AppError> {
        let file = File::open(path).map_err(|error| AppError::from_io(path, error))?;
        let decoder = MultiGzDecoder::new(BufReader::new(file));

        Ok(Self {
            path: label.to_path_buf(),
            backend: BamReaderBackend::Gzip(decoder),
        })
    }

    pub fn open_native_bgzf(path: &Path) -> Result<Self, AppError> {
        let reader = NativeBgzfReader::open(path)?;

        Ok(Self {
            path: path.to_path_buf(),
            backend: BamReaderBackend::NativeBgzf(reader),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn virtual_offset(&self) -> Result<Option<VirtualOffset>, AppError> {
        match &self.backend {
            BamReaderBackend::Gzip(_) => Ok(None),
            BamReaderBackend::NativeBgzf(reader) => reader.virtual_offset().map(Some),
        }
    }

    pub fn seek_virtual_offset(&mut self, offset: VirtualOffset) -> Result<(), AppError> {
        match &mut self.backend {
            BamReaderBackend::Gzip(_) => Err(AppError::InvalidBam {
                path: self.path.clone(),
                detail: "BAM reader backend does not support BGZF virtual-offset seeking."
                    .to_string(),
            }),
            BamReaderBackend::NativeBgzf(reader) => reader.seek_virtual_offset(offset),
        }
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

    pub fn read_i32_le_with_context(&mut self, detail: &'static str) -> Result<i32, AppError> {
        let mut bytes = [0_u8; 4];
        self.read_exact_into_with_context(&mut bytes, detail)?;
        Ok(i32::from_le_bytes(bytes))
    }

    pub fn read_optional_i32_le(&mut self) -> Result<Option<i32>, AppError> {
        let mut bytes = [0_u8; 4];
        match self.read_from_backend(&mut bytes[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(error) => return Err(error),
        }

        self.read_exact_into_with_context(
            &mut bytes[1..],
            "BAM stream ended while reading the next record block size.",
        )?;

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

    pub fn read_exact_vec_with_context(
        &mut self,
        len: usize,
        detail: &'static str,
    ) -> Result<Vec<u8>, AppError> {
        let mut buffer = vec![0_u8; len];
        self.read_exact_into_with_context(&mut buffer, detail)?;
        Ok(buffer)
    }

    pub(crate) fn discard_exact_with_context(
        &mut self,
        len: usize,
        detail: &'static str,
    ) -> Result<(), AppError> {
        match &mut self.backend {
            BamReaderBackend::NativeBgzf(reader) => reader.discard(len).map_err(|error| {
                if matches!(error, AppError::TruncatedFile { .. }) {
                    AppError::TruncatedFile {
                        path: self.path.clone(),
                        detail: detail.to_string(),
                    }
                } else {
                    error
                }
            }),
            BamReaderBackend::Gzip(_) => {
                let mut remaining = len;
                let mut buffer = [0_u8; 8192];
                while remaining > 0 {
                    let chunk = remaining.min(buffer.len());
                    self.read_exact_into_with_context(&mut buffer[..chunk], detail)?;
                    remaining -= chunk;
                }
                Ok(())
            }
        }
    }

    fn read_exact_into(&mut self, buffer: &mut [u8]) -> Result<(), AppError> {
        self.read_exact_into_with_context(
            buffer,
            "BAM stream ended before the expected number of bytes were available.",
        )
    }

    fn read_exact_into_with_context(
        &mut self,
        buffer: &mut [u8],
        detail: &'static str,
    ) -> Result<(), AppError> {
        let mut bytes_read = 0;
        while bytes_read < buffer.len() {
            match self.read_from_backend(&mut buffer[bytes_read..]) {
                Ok(0) => {
                    return Err(AppError::TruncatedFile {
                        path: self.path.clone(),
                        detail: detail.to_string(),
                    });
                }
                Ok(count) => bytes_read += count,
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    fn read_from_backend(&mut self, buffer: &mut [u8]) -> Result<usize, AppError> {
        match &mut self.backend {
            BamReaderBackend::Gzip(decoder) => decoder
                .read(buffer)
                .map_err(|error| AppError::from_io(&self.path, error)),
            BamReaderBackend::NativeBgzf(reader) => reader.read(buffer),
        }
    }
}
