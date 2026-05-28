use super::TypeError;
use crate::pymergetic::cruspy::utils::fourcc;

pub const META_TYPE_MAGIC: u32 = fourcc::fourcc("MTYP");
pub const META_TYPE_VERSION: u16 = 1;
pub const META_TYPE_HEADER_LEN: usize = 32;

/// Runtime descriptor for a registered type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaType {
    pub type_name: String,
    pub type_uuid: [u8; 16],
    pub type_schema_version: u32,
    pub flags: u32,
}

pub trait HasMetaType {
    const TYPE_NAME: &'static str;
    const TYPE_UUID: [u8; 16];
    const TYPE_SCHEMA_VERSION: u32;
}

impl MetaType {
    pub fn from_type<T: HasMetaType>() -> Self {
        Self::new(T::TYPE_NAME, T::TYPE_UUID, T::TYPE_SCHEMA_VERSION)
    }

    pub fn new(
        type_name: impl Into<String>,
        type_uuid: [u8; 16],
        type_schema_version: u32,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            type_uuid,
            type_schema_version,
            flags: 0,
        }
    }

    pub fn to_header(&self) -> MetaTypeHeader {
        MetaTypeHeader {
            magic: META_TYPE_MAGIC,
            version: META_TYPE_VERSION,
            _reserved: 0,
            type_uuid: self.type_uuid,
            type_schema_version: self.type_schema_version,
            flags: self.flags,
        }
    }
}

/// On-segment fixed-size header row in the meta-type partition.
///
/// Human-readable names are not stored here; resolve via [`HasMetaType::TYPE_NAME`] or tooling.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MetaTypeHeader {
    pub magic: u32,
    pub version: u16,
    pub _reserved: u16,
    pub type_uuid: [u8; 16],
    pub type_schema_version: u32,
    pub flags: u32,
}

impl MetaTypeHeader {
    pub fn validate(&self) -> Result<(), TypeError> {
        if self.magic != META_TYPE_MAGIC || self.version != META_TYPE_VERSION {
            return Err(TypeError::BadHeader);
        }
        Ok(())
    }

    pub fn encode_into(self, dst: &mut [u8]) -> Result<(), TypeError> {
        if dst.len() < META_TYPE_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        dst[0..4].copy_from_slice(&self.magic.to_le_bytes());
        dst[4..6].copy_from_slice(&self.version.to_le_bytes());
        dst[6..8].copy_from_slice(&self._reserved.to_le_bytes());
        dst[8..24].copy_from_slice(&self.type_uuid);
        dst[24..28].copy_from_slice(&self.type_schema_version.to_le_bytes());
        dst[28..32].copy_from_slice(&self.flags.to_le_bytes());
        Ok(())
    }

    pub fn decode_from(src: &[u8]) -> Result<Self, TypeError> {
        if src.len() < META_TYPE_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&src[8..24]);
        let h = Self {
            magic: u32::from_le_bytes([src[0], src[1], src[2], src[3]]),
            version: u16::from_le_bytes([src[4], src[5]]),
            _reserved: u16::from_le_bytes([src[6], src[7]]),
            type_uuid: uuid,
            type_schema_version: u32::from_le_bytes([src[24], src[25], src[26], src[27]]),
            flags: u32::from_le_bytes([src[28], src[29], src[30], src[31]]),
        };
        h.validate()?;
        Ok(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metatype_header_roundtrip() {
        let mt = MetaType::new("cruspy.types.FlexString", *b"0123456789ABCDEF", 3);
        let h = mt.to_header();
        let mut buf = [0u8; META_TYPE_HEADER_LEN];
        h.encode_into(&mut buf).unwrap();
        let decoded = MetaTypeHeader::decode_from(&buf).unwrap();
        assert_eq!(decoded, h);
        assert_eq!(decoded.type_uuid, mt.type_uuid);
    }
}
