//! Wire header for one row in the object-instance catalog (`COBJ`).

use super::TypeError;
use crate::pymergetic::cruspy::memory::wire::tags::record;

/// Record layout guard ([`record::OBJH`]).
pub const OBJECT_HEADER_MAGIC: u32 = record::OBJH;
pub const OBJECT_HEADER_VERSION: u16 = 1;
pub const OBJECT_HEADER_LEN: usize = 32;

/// One registered object instance (points at heap payload via [`Self::data_offset`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObjectHeader {
    pub magic: u32,
    pub version: u16,
    pub _reserved: u16,
    /// Index into the type catalog (`CTLG`).
    pub type_index: u32,
    /// Stable index within this segment's object table (COBJ row number).
    pub object_index: u32,
    /// Arena-relative offset to instance body (e.g. `STRS` blob) in talc heap.
    pub data_offset: u32,
    pub flags: u32,
}

impl ObjectHeader {
    pub fn new(type_index: u32, object_index: u32, data_offset: u32) -> Self {
        Self {
            magic: OBJECT_HEADER_MAGIC,
            version: OBJECT_HEADER_VERSION,
            _reserved: 0,
            type_index,
            object_index,
            data_offset,
            flags: 0,
        }
    }

    pub fn validate(&self) -> Result<(), TypeError> {
        if self.magic != OBJECT_HEADER_MAGIC || self.version != OBJECT_HEADER_VERSION {
            return Err(TypeError::BadHeader);
        }
        Ok(())
    }

    pub fn encode_into(self, dst: &mut [u8]) -> Result<(), TypeError> {
        if dst.len() < OBJECT_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        dst[0..4].copy_from_slice(&self.magic.to_le_bytes());
        dst[4..6].copy_from_slice(&self.version.to_le_bytes());
        dst[6..8].copy_from_slice(&self._reserved.to_le_bytes());
        dst[8..12].copy_from_slice(&self.type_index.to_le_bytes());
        dst[12..16].copy_from_slice(&self.object_index.to_le_bytes());
        dst[16..20].copy_from_slice(&self.data_offset.to_le_bytes());
        dst[20..24].copy_from_slice(&self.flags.to_le_bytes());
        dst[24..OBJECT_HEADER_LEN].fill(0);
        Ok(())
    }

    pub fn decode_from(src: &[u8]) -> Result<Self, TypeError> {
        if src.len() < OBJECT_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        let h = Self {
            magic: u32::from_le_bytes([src[0], src[1], src[2], src[3]]),
            version: u16::from_le_bytes([src[4], src[5]]),
            _reserved: u16::from_le_bytes([src[6], src[7]]),
            type_index: u32::from_le_bytes([src[8], src[9], src[10], src[11]]),
            object_index: u32::from_le_bytes([src[12], src[13], src[14], src[15]]),
            data_offset: u32::from_le_bytes([src[16], src[17], src[18], src[19]]),
            flags: u32::from_le_bytes([src[20], src[21], src[22], src[23]]),
        };
        h.validate()?;
        Ok(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_header_roundtrip() {
        let h = ObjectHeader::new(1, 7, 4096);
        let mut buf = [0u8; OBJECT_HEADER_LEN];
        h.encode_into(&mut buf).unwrap();
        let decoded = ObjectHeader::decode_from(&buf).unwrap();
        assert_eq!(decoded, h);
    }
}
