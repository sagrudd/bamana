//! BGZF virtual offsets.
//!
//! A BGZF virtual offset is the packed offset representation used by BAM index
//! formats: the high 48 bits identify the compressed BGZF block start in the
//! file, and the low 16 bits identify an uncompressed byte offset inside that
//! block. Keeping this as an explicit type prevents new BGZF and indexing code
//! from passing ambiguous raw integers across module boundaries.

use std::{error::Error, fmt};

pub const MAX_COMPRESSED_BLOCK_OFFSET: u64 = (1_u64 << 48) - 1;
pub const MAX_UNCOMPRESSED_BLOCK_OFFSET: u32 = u16::MAX as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualOffsetError {
    CompressedBlockOffsetOutOfRange { value: u64 },
    UncompressedBlockOffsetOutOfRange { value: u32 },
}

impl fmt::Display for VirtualOffsetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompressedBlockOffsetOutOfRange { value } => write!(
                formatter,
                "compressed BGZF block offset {value} exceeds the 48-bit virtual-offset limit"
            ),
            Self::UncompressedBlockOffsetOutOfRange { value } => write!(
                formatter,
                "uncompressed BGZF in-block offset {value} exceeds the 16-bit virtual-offset limit"
            ),
        }
    }
}

impl Error for VirtualOffsetError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualOffset {
    compressed_block_offset: u64,
    uncompressed_block_offset: u16,
}

impl VirtualOffset {
    pub const ZERO: Self = Self {
        compressed_block_offset: 0,
        uncompressed_block_offset: 0,
    };

    pub fn new(
        compressed_block_offset: u64,
        uncompressed_block_offset: u32,
    ) -> Result<Self, VirtualOffsetError> {
        if compressed_block_offset > MAX_COMPRESSED_BLOCK_OFFSET {
            return Err(VirtualOffsetError::CompressedBlockOffsetOutOfRange {
                value: compressed_block_offset,
            });
        }

        if uncompressed_block_offset > MAX_UNCOMPRESSED_BLOCK_OFFSET {
            return Err(VirtualOffsetError::UncompressedBlockOffsetOutOfRange {
                value: uncompressed_block_offset,
            });
        }

        Ok(Self {
            compressed_block_offset,
            uncompressed_block_offset: uncompressed_block_offset as u16,
        })
    }

    pub fn from_packed(value: u64) -> Self {
        Self {
            compressed_block_offset: value >> 16,
            uncompressed_block_offset: value as u16,
        }
    }

    pub fn packed(self) -> u64 {
        (self.compressed_block_offset << 16) | u64::from(self.uncompressed_block_offset)
    }

    pub fn compressed_block_offset(self) -> u64 {
        self.compressed_block_offset
    }

    pub fn uncompressed_block_offset(self) -> u16 {
        self.uncompressed_block_offset
    }
}

impl From<VirtualOffset> for u64 {
    fn from(value: VirtualOffset) -> Self {
        value.packed()
    }
}

impl From<u64> for VirtualOffset {
    fn from(value: u64) -> Self {
        Self::from_packed(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_COMPRESSED_BLOCK_OFFSET, MAX_UNCOMPRESSED_BLOCK_OFFSET, VirtualOffset,
        VirtualOffsetError,
    };

    #[test]
    fn packs_and_unpacks_components() {
        let offset = VirtualOffset::new(0x0123_4567_89ab, 0xcdef).expect("offset should fit");

        assert_eq!(offset.compressed_block_offset(), 0x0123_4567_89ab);
        assert_eq!(offset.uncompressed_block_offset(), 0xcdef);
        assert_eq!(offset.packed(), 0x0123_4567_89ab_cdef);
        assert_eq!(VirtualOffset::from_packed(offset.packed()), offset);
    }

    #[test]
    fn supports_boundary_values() {
        let zero = VirtualOffset::ZERO;
        assert_eq!(zero.packed(), 0);

        let maximum =
            VirtualOffset::new(MAX_COMPRESSED_BLOCK_OFFSET, MAX_UNCOMPRESSED_BLOCK_OFFSET)
                .expect("maximum virtual offset should fit");

        assert_eq!(maximum.compressed_block_offset(), 0x0000_ffff_ffff_ffff);
        assert_eq!(maximum.uncompressed_block_offset(), u16::MAX);
        assert_eq!(maximum.packed(), u64::MAX);
        assert_eq!(VirtualOffset::from_packed(u64::MAX), maximum);
    }

    #[test]
    fn rejects_out_of_range_components() {
        assert_eq!(
            VirtualOffset::new(MAX_COMPRESSED_BLOCK_OFFSET + 1, 0),
            Err(VirtualOffsetError::CompressedBlockOffsetOutOfRange {
                value: MAX_COMPRESSED_BLOCK_OFFSET + 1,
            })
        );

        assert_eq!(
            VirtualOffset::new(0, MAX_UNCOMPRESSED_BLOCK_OFFSET + 1),
            Err(VirtualOffsetError::UncompressedBlockOffsetOutOfRange {
                value: MAX_UNCOMPRESSED_BLOCK_OFFSET + 1,
            })
        );
    }

    #[test]
    fn orders_by_packed_bgzf_position() {
        let mut offsets = [
            VirtualOffset::new(20, 0).expect("offset should fit"),
            VirtualOffset::new(10, 5).expect("offset should fit"),
            VirtualOffset::new(10, 2).expect("offset should fit"),
            VirtualOffset::new(0, u16::MAX as u32).expect("offset should fit"),
        ];

        offsets.sort();

        let packed: Vec<u64> = offsets.into_iter().map(VirtualOffset::packed).collect();
        assert_eq!(
            packed,
            vec![
                0x0000_0000_0000_ffff,
                0x0000_0000_000a_0002,
                0x0000_0000_000a_0005,
                0x0000_0000_0014_0000,
            ]
        );
    }
}
