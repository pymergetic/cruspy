//! Type catalog blob at a fixed arena offset (primary slab).

use crate::pymergetic::cruspy::memory::types::{
    HasMetaType, MetaType, MetaTypeHeader, TypeError, META_TYPE_HEADER_LEN,
};
use crate::pymergetic::cruspy::utils::{fourcc, uuid::Uuid};

pub const TYPE_CATALOG_MAGIC: u32 = fourcc::fourcc("CTLG");
pub const TYPE_CATALOG_VERSION: u32 = 4;
pub const TYPE_CATALOG_HEADER_LEN: usize = 16;

/// Default number of [`MetaTypeHeader`] row slots reserved in the pinned talc catalog.
pub const DEFAULT_TYPE_CATALOG_CAPACITY: u32 = 64;

/// Row index of the catalog's own [`MetaTypeHeader`] (always registered first on mount).
pub const TYPE_CATALOG_SELF_INDEX: u32 = 0;

/// On-segment catalog: [`MetaTypeHeader`] rows; row index is the compact `type_index`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeCatalog {
    pub types: Vec<MetaTypeHeader>,
    pub capacity: u32,
}

impl HasMetaType for TypeCatalog {
    const TYPE_NAME: &'static str = "cruspy.memory.TypeCatalog";
    const TYPE_UUID: [u8; 16] = Uuid::must_parse("c4a8e910-2f3b-4d61-9c07-1e5b29384d01").bytes();
    const TYPE_SCHEMA_VERSION: u32 = 1;
}

impl Default for TypeCatalog {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_TYPE_CATALOG_CAPACITY)
    }
}

impl TypeCatalog {
    pub fn new(types: Vec<MetaTypeHeader>) -> Self {
        let capacity = types.len().try_into().unwrap_or(u32::MAX);
        Self { types, capacity }
    }

    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            types: Vec::new(),
            capacity,
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Primary-slab mount: reserved slots plus this catalog registered at [`TYPE_CATALOG_SELF_INDEX`].
    pub fn for_mount(capacity: u32) -> Result<Self, TypeError> {
        let mut catalog = Self::with_capacity(capacity);
        catalog.register_self_type()?;
        Ok(catalog)
    }

    /// Register [`TypeCatalog`] as a type row (used for the mandatory first entry on mount).
    pub fn register_self_type(&mut self) -> Result<u32, TypeError> {
        let row = MetaType::from_type::<Self>().to_header();
        self.append_type(row)
    }

    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    pub fn slots_remaining(&self) -> usize {
        self.capacity as usize - self.types.len()
    }

    pub fn encoded_len_used(type_count: usize) -> usize {
        TYPE_CATALOG_HEADER_LEN + type_count * META_TYPE_HEADER_LEN
    }

    pub fn encoded_len_reserved(capacity: usize) -> usize {
        TYPE_CATALOG_HEADER_LEN + capacity * META_TYPE_HEADER_LEN
    }

    /// Bytes of valid CTLG wire data (header + registered rows).
    pub fn used_len(&self) -> usize {
        Self::encoded_len_used(self.types.len())
    }

    /// Bytes reserved in the pinned talc allocation (header + all row slots).
    pub fn allocated_len(&self) -> usize {
        Self::encoded_len_reserved(self.capacity as usize)
    }

    pub fn append_type(&mut self, row: MetaTypeHeader) -> Result<u32, TypeError> {
        if self.types.len() >= self.capacity as usize {
            return Err(TypeError::CapacityExceeded);
        }
        row.validate()?;
        let index = u32::try_from(self.types.len()).map_err(|_| TypeError::CapacityExceeded)?;
        self.types.push(row);
        Ok(index)
    }

    pub fn write_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        if self.types.len() > self.capacity as usize {
            return Err(TypeError::CapacityExceeded);
        }
        let need = self.allocated_len();
        if dst.len() < need {
            return Err(TypeError::OutOfBounds);
        }
        dst[0..4].copy_from_slice(&TYPE_CATALOG_MAGIC.to_le_bytes());
        dst[4..8].copy_from_slice(&TYPE_CATALOG_VERSION.to_le_bytes());
        dst[8..12].copy_from_slice(&(self.types.len() as u32).to_le_bytes());
        dst[12..16].copy_from_slice(&self.capacity.to_le_bytes());
        let mut off = TYPE_CATALOG_HEADER_LEN;
        for row in &self.types {
            row.encode_into(&mut dst[off..off + META_TYPE_HEADER_LEN])?;
            off += META_TYPE_HEADER_LEN;
        }
        Ok(())
    }

    pub fn read_from(src: &[u8]) -> Result<Self, TypeError> {
        if src.len() < TYPE_CATALOG_HEADER_LEN {
            return Err(TypeError::OutOfBounds);
        }
        let magic = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
        let version = u32::from_le_bytes([src[4], src[5], src[6], src[7]]);
        let count = u32::from_le_bytes([src[8], src[9], src[10], src[11]]) as usize;
        let capacity = u32::from_le_bytes([src[12], src[13], src[14], src[15]]);
        if magic != TYPE_CATALOG_MAGIC || version != TYPE_CATALOG_VERSION {
            return Err(TypeError::BadHeader);
        }
        let capacity_usize = capacity as usize;
        if count > capacity_usize {
            return Err(TypeError::BadHeader);
        }
        let need = Self::encoded_len_reserved(capacity_usize);
        if src.len() < need {
            return Err(TypeError::OutOfBounds);
        }
        let mut types = Vec::with_capacity(count);
        let mut off = TYPE_CATALOG_HEADER_LEN;
        for _ in 0..count {
            types.push(MetaTypeHeader::decode_from(&src[off..off + META_TYPE_HEADER_LEN])?);
            off += META_TYPE_HEADER_LEN;
        }
        Ok(Self { types, capacity })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::memory::types::MetaType;

    #[test]
    fn catalog_roundtrip() {
        let mt = MetaType::from_type::<crate::pymergetic::cruspy::memory::types::FlexString>();
        let row = mt.to_header();
        let mut cat = TypeCatalog::with_capacity(8);
        cat.append_type(row).unwrap();
        let mut buf = vec![0u8; cat.allocated_len()];
        cat.write_into(&mut buf).unwrap();
        let decoded = TypeCatalog::read_from(&buf).unwrap();
        assert_eq!(decoded.capacity, 8);
        assert_eq!(decoded.types.len(), 1);
        assert_eq!(decoded.types[0], row);
    }

    #[test]
    fn catalog_for_mount_registers_self_at_zero() {
        let cat = TypeCatalog::for_mount(8).unwrap();
        assert_eq!(cat.types.len(), 1);
        assert_eq!(
            cat.types[TYPE_CATALOG_SELF_INDEX as usize],
            MetaType::from_type::<TypeCatalog>().to_header()
        );
    }

    #[test]
    fn catalog_append_until_full() {
        let mt = MetaType::from_type::<crate::pymergetic::cruspy::memory::types::FlexString>();
        let row = mt.to_header();
        let mut cat = TypeCatalog::with_capacity(2);
        assert_eq!(cat.append_type(row).unwrap(), 0);
        assert_eq!(cat.append_type(row).unwrap(), 1);
        assert!(matches!(
            cat.append_type(row),
            Err(TypeError::CapacityExceeded)
        ));
    }
}
