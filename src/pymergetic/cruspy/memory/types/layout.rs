use super::TypeError;

use crate::pymergetic::cruspy::utils::fourcc;

pub const STRING_MAGIC: u32 = fourcc::fourcc("STRS");
pub const STRING_VERSION: u16 = 1;
pub const STRING_HEADER_LEN: usize = 16;

/// On-wire header for a variable-length UTF-8 string.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StringHeader {
    pub magic: u32,
    pub version: u16,
    pub _reserved: u16,
    pub len: u32,
    pub capacity: u32,
}

impl StringHeader {
    pub fn new(len: u32, capacity: u32) -> Self {
        Self {
            magic: STRING_MAGIC,
            version: STRING_VERSION,
            _reserved: 0,
            len,
            capacity,
        }
    }

    pub fn payload_range(&self, base_offset: usize) -> Result<std::ops::Range<usize>, TypeError> {
        let start = base_offset
            .checked_add(STRING_HEADER_LEN)
            .ok_or(TypeError::OutOfBounds)?;
        let end = start
            .checked_add(self.capacity as usize)
            .ok_or(TypeError::OutOfBounds)?;
        Ok(start..end)
    }

    pub fn validate(&self) -> Result<(), TypeError> {
        if self.magic != STRING_MAGIC || self.version != STRING_VERSION || self.len > self.capacity {
            return Err(TypeError::BadHeader);
        }
        Ok(())
    }

    pub fn encode_into(self, dst: &mut [u8]) -> Result<(), TypeError> {
        if dst.len() < STRING_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        dst[0..4].copy_from_slice(&self.magic.to_le_bytes());
        dst[4..6].copy_from_slice(&self.version.to_le_bytes());
        dst[6..8].copy_from_slice(&self._reserved.to_le_bytes());
        dst[8..12].copy_from_slice(&self.len.to_le_bytes());
        dst[12..16].copy_from_slice(&self.capacity.to_le_bytes());
        Ok(())
    }

    pub fn decode_from(src: &[u8]) -> Result<Self, TypeError> {
        if src.len() < STRING_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        let h = Self {
            magic: u32::from_le_bytes([src[0], src[1], src[2], src[3]]),
            version: u16::from_le_bytes([src[4], src[5]]),
            _reserved: u16::from_le_bytes([src[6], src[7]]),
            len: u32::from_le_bytes([src[8], src[9], src[10], src[11]]),
            capacity: u32::from_le_bytes([src[12], src[13], src[14], src[15]]),
        };
        h.validate()?;
        Ok(h)
    }
}
