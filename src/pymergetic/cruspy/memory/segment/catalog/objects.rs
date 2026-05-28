//! Object-instance catalog (`COBJ`): [`ObjectHeader`] rows.

use crate::pymergetic::cruspy::memory::types::{ObjectHeader, TypeError, OBJECT_HEADER_LEN};
use crate::pymergetic::cruspy::memory::wire::tags::catalog;

use super::pin::PinnedCatalog;
use super::wire::{Catalog, CatalogKind, CatalogRow};

pub const OBJECT_CATALOG_MAGIC: u32 = catalog::COBJ;
pub const OBJECT_CATALOG_VERSION: u32 = 2;
pub const OBJECT_CATALOG_HEADER_LEN: usize = super::wire::CATALOG_HEADER_LEN;
pub const DEFAULT_OBJECT_CATALOG_CAPACITY: u32 = 4096;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObjectCatalogKind;

impl CatalogKind for ObjectCatalogKind {
    const MAGIC: u32 = OBJECT_CATALOG_MAGIC;
    const VERSION: u32 = OBJECT_CATALOG_VERSION;
    const DEFAULT_CAPACITY: u32 = DEFAULT_OBJECT_CATALOG_CAPACITY;
    type Row = ObjectHeader;
}

impl CatalogRow for ObjectHeader {
    fn row_len() -> usize {
        OBJECT_HEADER_LEN
    }

    fn encode_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        ObjectHeader::encode_into(*self, dst)
    }

    fn decode_from(src: &[u8]) -> Result<Self, TypeError> {
        ObjectHeader::decode_from(src)
    }

    fn validate(&self) -> Result<(), TypeError> {
        ObjectHeader::validate(self)
    }
}

/// Object table on the primary slab; row index is the compact `object_index`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectCatalog(Catalog<ObjectCatalogKind>);

impl Default for ObjectCatalog {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_OBJECT_CATALOG_CAPACITY)
    }
}

impl ObjectCatalog {
    pub fn with_capacity(capacity: u32) -> Self {
        Self(Catalog::with_capacity(capacity))
    }

    /// Extension chunk: empty table, same reserved capacity as the tail being extended.
    pub fn for_mount_extension(capacity: u32) -> Result<Self, TypeError> {
        Ok(Self::with_capacity(capacity))
    }

    /// Primary mount: reserved slots, initially empty (`object_count == 0`).
    pub fn for_mount(capacity: u32) -> Result<Self, TypeError> {
        Self::for_mount_extension(capacity)
    }

    pub fn from_flat(catalog: Catalog<ObjectCatalogKind>) -> Self {
        Self(catalog)
    }

    pub fn inner(&self) -> &Catalog<ObjectCatalogKind> {
        &self.0
    }

    pub fn objects(&self) -> &[ObjectHeader] {
        &self.0.rows
    }

    pub fn object_count(&self) -> usize {
        self.0.count()
    }

    pub fn capacity(&self) -> u32 {
        self.0.capacity
    }

    pub fn slots_remaining(&self) -> usize {
        self.0.slots_remaining()
    }

    pub fn used_len(&self) -> usize {
        self.0.used_len()
    }

    pub fn allocated_len(&self) -> usize {
        self.0.allocated_len()
    }

    pub fn append_object(&mut self, row: ObjectHeader) -> Result<u32, TypeError> {
        self.0.append(row)
    }

    pub fn get(&self, object_index: u32) -> Option<&ObjectHeader> {
        self.objects().get(object_index as usize)
    }

    pub fn write_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        self.0.write_into(dst)
    }

    pub fn read_from(src: &[u8]) -> Result<Self, TypeError> {
        Catalog::read_from(src).map(Self)
    }
}

impl PinnedCatalog for ObjectCatalog {
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

    #[test]
    fn object_catalog_append_sets_object_index() {
        let row = ObjectHeader::new(1, 0, 512);
        let mut cat = ObjectCatalog::with_capacity(4);
        assert_eq!(cat.append_object(row).unwrap(), 0);
        assert_eq!(cat.objects()[0].object_index, 0);
    }

    #[test]
    fn object_catalog_roundtrip() {
        let row = ObjectHeader::new(1, 0, 512);
        let mut cat = ObjectCatalog::with_capacity(4);
        cat.append_object(row).unwrap();
        let mut buf = vec![0u8; cat.allocated_len()];
        cat.write_into(&mut buf).unwrap();
        let decoded = ObjectCatalog::read_from(&buf).unwrap();
        assert_eq!(decoded.object_count(), 1);
        assert_eq!(decoded.objects()[0], row);
    }
}
