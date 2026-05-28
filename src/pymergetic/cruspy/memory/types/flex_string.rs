use super::{HasMetaType, StringHeader, TypeError, TypeHandle, STRING_HEADER_LEN};
use crate::pymergetic::cruspy::utils::uuid::Uuid;

/// First scaffolded concrete type living in slab memory.
///
/// This type is allocator-agnostic: it only needs a byte arena and a handle.
/// Manager/talc integration can build on top by allocating offsets and filling
/// `TypeHandle`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FlexString {
    pub handle: TypeHandle,
}

impl HasMetaType for FlexString {
    const TYPE_NAME: &'static str = "cruspy.types.FlexString";
    const TYPE_UUID: [u8; 16] = Uuid::must_parse("8f3c2154-9a4e-074b-2cb1-6d18440a7f92").bytes();
    const TYPE_SCHEMA_VERSION: u32 = 1;
}

impl FlexString {
    pub fn new(handle: TypeHandle) -> Self {
        Self { handle }
    }

    pub fn required_len(capacity: usize) -> usize {
        STRING_HEADER_LEN.saturating_add(capacity)
    }

    pub fn init_in(&self, arena: &mut [u8], capacity: usize) -> Result<(), TypeError> {
        let header = StringHeader::new(0, capacity as u32);
        let range = self.header_range(arena.len())?;
        header.encode_into(&mut arena[range])?;
        Ok(())
    }

    pub fn set(&self, arena: &mut [u8], value: &str) -> Result<(), TypeError> {
        let header_range = self.header_range(arena.len())?;
        let mut header = StringHeader::decode_from(&arena[header_range.clone()])?;
        if value.len() > header.capacity as usize {
            return Err(TypeError::CapacityExceeded);
        }
        let payload_range = header.payload_range(self.handle.offset)?;
        if payload_range.end > self.handle.end() || payload_range.end > arena.len() {
            return Err(TypeError::OutOfBounds);
        }
        let payload = &mut arena[payload_range];
        payload.fill(0);
        payload[..value.len()].copy_from_slice(value.as_bytes());
        header.len = value.len() as u32;
        header.encode_into(&mut arena[header_range])?;
        Ok(())
    }

    pub fn get(&self, arena: &[u8]) -> Result<String, TypeError> {
        let header_range = self.header_range(arena.len())?;
        let header = StringHeader::decode_from(&arena[header_range])?;
        let payload_range = header.payload_range(self.handle.offset)?;
        if payload_range.end > self.handle.end() || payload_range.end > arena.len() {
            return Err(TypeError::OutOfBounds);
        }
        let bytes = &arena[payload_range.start..payload_range.start + header.len as usize];
        std::str::from_utf8(bytes)
            .map(|s| s.to_owned())
            .map_err(|_| TypeError::InvalidUtf8)
    }

    fn header_range(&self, arena_len: usize) -> Result<std::ops::Range<usize>, TypeError> {
        let start = self.handle.offset;
        let end = start
            .checked_add(STRING_HEADER_LEN)
            .ok_or(TypeError::OutOfBounds)?;
        if end > self.handle.end() || end > arena_len {
            return Err(TypeError::OutOfBounds);
        }
        Ok(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::memory::types::{MemoryObject, MetaType};
    use crate::pymergetic::cruspy::memory::manager::{Id, SegmentId};

    #[test]
    fn has_meta_type_constants() {
        let mt = MetaType::from_type::<FlexString>();
        assert_eq!(mt.type_uuid, FlexString::TYPE_UUID);
        assert_eq!(mt.type_name, FlexString::TYPE_NAME);
        assert_eq!(mt.type_schema_version, FlexString::TYPE_SCHEMA_VERSION);
    }

    #[test]
    fn roundtrip_string_in_arena() {
        let mut arena = vec![0u8; 256];
        let obj = MemoryObject::new(Id(1), SegmentId(2));
        let h = TypeHandle::new(obj, 0, 32, FlexString::required_len(64));
        let s = FlexString::new(h);
        s.init_in(&mut arena, 64).unwrap();
        s.set(&mut arena, "hello slabs").unwrap();
        assert_eq!(s.get(&arena).unwrap(), "hello slabs");
    }
}
