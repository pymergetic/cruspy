//! [`Segment`] API for the object catalog (`COBJ`).

use crate::pymergetic::cruspy::memory::segment::Segment;
use crate::pymergetic::cruspy::memory::segment::SegmentError;
use crate::pymergetic::cruspy::memory::types::{ObjectHeader, TypeError};

use super::chain::{append_to_chain, chain_chunk_count, flatten_chain, get_in_chain, load_chain};
use super::objects::ObjectCatalogKind;
use super::primary::primary_header;
use super::{map_type_err, ObjectCatalog};

impl Segment {
    /// Flattened logical view of the full object chain (all chunks).
    pub fn object_catalog(&self) -> Result<ObjectCatalog, SegmentError> {
        let h = primary_header(self)?;
        let chunks = load_chain::<ObjectCatalogKind>(
            self,
            h.object_catalog_offset,
            h.object_catalog_len,
        )?;
        Ok(ObjectCatalog::from_flat(flatten_chain(&chunks)))
    }

    /// Head COBJ chunk only (preserves `next_offset` / `next_len` on the wire).
    pub fn object_catalog_head(&self) -> Result<ObjectCatalog, SegmentError> {
        let h = primary_header(self)?;
        let bytes = super::primary::blob(
            self,
            h.object_catalog_offset,
            h.object_catalog_len,
        )?;
        ObjectCatalog::read_from(bytes).map_err(map_type_err)
    }

    pub fn object_catalog_chunk_count(&self) -> Result<usize, SegmentError> {
        let h = primary_header(self)?;
        chain_chunk_count::<ObjectCatalogKind>(
            self,
            h.object_catalog_offset,
            h.object_catalog_len,
        )
    }

    pub fn with_object_catalog_mut<R>(
        &mut self,
        f: impl FnOnce(&mut ObjectCatalog) -> Result<R, TypeError>,
    ) -> Result<R, SegmentError> {
        let h = primary_header(self)?;
        let chunks = load_chain::<ObjectCatalogKind>(
            self,
            h.object_catalog_offset,
            h.object_catalog_len,
        )?;
        let mut view = ObjectCatalog::from_flat(flatten_chain(&chunks));
        let out = f(&mut view).map_err(map_type_err)?;
        super::chain::redistribute_chain(self, &chunks, view.inner())?;
        Ok(out)
    }

    pub fn with_object_catalog<R>(
        &self,
        f: impl FnOnce(&ObjectCatalog) -> Result<R, TypeError>,
    ) -> Result<R, SegmentError> {
        let catalog = self.object_catalog()?;
        f(&catalog).map_err(map_type_err)
    }

    pub fn register_object(&mut self, row: ObjectHeader) -> Result<u32, SegmentError> {
        let h = primary_header(self)?;
        append_to_chain::<ObjectCatalogKind, _>(
            self,
            h.object_catalog_offset,
            h.object_catalog_len,
            row,
            |cap| ObjectCatalog::for_mount_extension(cap).map_err(map_type_err),
        )
    }

    pub fn object_at(&self, object_index: u32) -> Result<Option<ObjectHeader>, SegmentError> {
        let h = primary_header(self)?;
        get_in_chain::<ObjectCatalogKind>(
            self,
            h.object_catalog_offset,
            h.object_catalog_len,
            object_index,
        )
    }
}
