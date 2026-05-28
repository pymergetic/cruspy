//! Metatype catalog (`CTLG`): [`MetaTypeHeader`] rows on the primary slab.

use crate::pymergetic::cruspy::memory::types::{
    HasMetaType, MetaType, MetaTypeHeader, TypeError, META_TYPE_HEADER_LEN,
};
use crate::pymergetic::cruspy::memory::wire::tags::catalog;
use crate::pymergetic::cruspy::utils::uuid::Uuid;

use super::pin::PinnedCatalog;
use super::wire::{Catalog, CatalogKind, CatalogRow};

pub const METATYPE_CATALOG_MAGIC: u32 = catalog::CTLG;
pub const METATYPE_CATALOG_VERSION: u32 = 5;
pub const METATYPE_CATALOG_HEADER_LEN: usize = super::wire::CATALOG_HEADER_LEN;
pub const DEFAULT_METATYPE_CATALOG_CAPACITY: u32 = 256;
pub const METATYPE_CATALOG_SELF_INDEX: u32 = 0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MetaTypeCatalogKind;

impl CatalogKind for MetaTypeCatalogKind {
    const MAGIC: u32 = METATYPE_CATALOG_MAGIC;
    const VERSION: u32 = METATYPE_CATALOG_VERSION;
    const DEFAULT_CAPACITY: u32 = DEFAULT_METATYPE_CATALOG_CAPACITY;
    type Row = MetaTypeHeader;
}

impl CatalogRow for MetaTypeHeader {
    fn row_len() -> usize {
        META_TYPE_HEADER_LEN
    }

    fn encode_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        MetaTypeHeader::encode_into(*self, dst)
    }

    fn decode_from(src: &[u8]) -> Result<Self, TypeError> {
        MetaTypeHeader::decode_from(src)
    }

    fn validate(&self) -> Result<(), TypeError> {
        MetaTypeHeader::validate(self)
    }
}

/// Metatype table; row index is `type_index`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaTypeCatalog(Catalog<MetaTypeCatalogKind>);

impl Default for MetaTypeCatalog {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_METATYPE_CATALOG_CAPACITY)
    }
}

impl HasMetaType for MetaTypeCatalog {
    const TYPE_NAME: &'static str = "cruspy.memory.MetaTypeCatalog";
    const TYPE_UUID: [u8; 16] = Uuid::must_parse("c4a8e910-2f3b-4d61-9c07-1e5b29384d01").bytes();
    const TYPE_SCHEMA_VERSION: u32 = 1;
}

impl MetaTypeCatalog {
    pub fn new(rows: Vec<MetaTypeHeader>) -> Self {
        Self(Catalog::new(rows))
    }

    pub fn with_capacity(capacity: u32) -> Self {
        Self(Catalog::with_capacity(capacity))
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn for_mount(capacity: u32) -> Result<Self, TypeError> {
        let mut catalog = Self::with_capacity(capacity);
        catalog.register_self()?;
        Ok(catalog)
    }

    pub fn register_self(&mut self) -> Result<u32, TypeError> {
        self.append(MetaType::from_type::<Self>().to_header())
    }

    pub fn inner(&self) -> &Catalog<MetaTypeCatalogKind> {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut Catalog<MetaTypeCatalogKind> {
        &mut self.0
    }

    pub fn metatypes(&self) -> &[MetaTypeHeader] {
        &self.0.rows
    }

    pub fn count(&self) -> usize {
        self.0.count()
    }

    pub fn slots_remaining(&self) -> usize {
        self.0.slots_remaining()
    }

    pub fn capacity(&self) -> u32 {
        self.0.capacity
    }

    pub fn used_len(&self) -> usize {
        self.0.used_len()
    }

    pub fn allocated_len(&self) -> usize {
        self.0.allocated_len()
    }

    pub fn append(&mut self, row: MetaTypeHeader) -> Result<u32, TypeError> {
        self.0.append(row)
    }

    pub fn register_for<T: HasMetaType>(&mut self) -> Result<u32, TypeError> {
        self.append(MetaType::from_type::<T>().to_header())
    }

    pub fn index_for_uuid(&self, type_uuid: [u8; 16]) -> Option<u32> {
        self.metatypes()
            .iter()
            .position(|row| row.type_uuid == type_uuid)
            .and_then(|i| u32::try_from(i).ok())
    }

    pub fn index_for<T: HasMetaType>(&self) -> Option<u32> {
        self.index_for_uuid(T::TYPE_UUID)
    }

    pub fn get(&self, type_index: u32) -> Option<&MetaTypeHeader> {
        self.metatypes().get(type_index as usize)
    }

    pub fn for_mount_extension(capacity: u32) -> Result<Self, TypeError> {
        Ok(Self::with_capacity(capacity))
    }

    pub fn from_flat(catalog: Catalog<MetaTypeCatalogKind>) -> Self {
        Self(catalog)
    }

    pub fn ensure_registered<T: HasMetaType>(&mut self) -> Result<u32, TypeError> {
        if let Some(index) = self.index_for::<T>() {
            return Ok(index);
        }
        self.register_for::<T>()
    }

    pub fn write_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        self.0.write_into(dst)
    }

    pub fn read_from(src: &[u8]) -> Result<Self, TypeError> {
        Catalog::read_from(src).map(Self)
    }
}

impl PinnedCatalog for MetaTypeCatalog {
    fn allocated_len(&self) -> usize {
        self.0.allocated_len()
    }

    fn write_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        self.0.write_into(dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymergetic::cruspy::memory::types::FlexString;

    #[test]
    fn metatype_catalog_roundtrip() {
        let row = MetaType::from_type::<FlexString>().to_header();
        let mut cat = MetaTypeCatalog::with_capacity(8);
        cat.append(row).unwrap();
        let mut buf = vec![0u8; cat.allocated_len()];
        cat.write_into(&mut buf).unwrap();
        let decoded = MetaTypeCatalog::read_from(&buf).unwrap();
        assert_eq!(decoded.capacity(), 8);
        assert_eq!(decoded.metatypes().len(), 1);
        assert_eq!(decoded.metatypes()[0], row);
    }

    #[test]
    fn for_mount_registers_self_at_zero() {
        let cat = MetaTypeCatalog::for_mount(8).unwrap();
        assert_eq!(cat.metatypes().len(), 1);
        assert_eq!(
            cat.metatypes()[METATYPE_CATALOG_SELF_INDEX as usize],
            MetaType::from_type::<MetaTypeCatalog>().to_header()
        );
    }

    #[test]
    fn ensure_registered_is_idempotent() {
        let mut cat = MetaTypeCatalog::for_mount(8).unwrap();
        let a = cat.ensure_registered::<FlexString>().unwrap();
        let b = cat.ensure_registered::<FlexString>().unwrap();
        assert_eq!(a, b);
        assert_eq!(cat.count(), 2);
    }

    #[test]
    fn append_until_full() {
        let row = MetaType::from_type::<FlexString>().to_header();
        let mut cat = MetaTypeCatalog::with_capacity(2);
        assert_eq!(cat.append(row).unwrap(), 0);
        assert_eq!(cat.append(row).unwrap(), 1);
        assert!(matches!(cat.append(row), Err(TypeError::CapacityExceeded)));
    }
}
