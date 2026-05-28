//! [`Segment`] API for the object catalog (`COBJ`).

use crate::pymergetic::cruspy::memory::segment::SegmentError;
use crate::pymergetic::cruspy::memory::types::{ObjectHeader, TypeError};
use crate::pymergetic::cruspy::memory::segment::Segment;

use super::primary::{blob, blob_mut, primary_header};
use super::{map_type_err, ObjectCatalog};

impl Segment {
    /// Read the pinned object catalog on the primary slab (decoded copy).
    pub fn object_catalog(&self) -> Result<ObjectCatalog, SegmentError> {
        let h = primary_header(self)?;
        let bytes = blob(self, h.object_catalog_offset, h.object_catalog_len)?;
        ObjectCatalog::read_from(bytes).map_err(map_type_err)
    }

    /// Decode → mutate → write back the live COBJ blob.
    pub fn with_object_catalog_mut<R>(
        &mut self,
        f: impl FnOnce(&mut ObjectCatalog) -> Result<R, TypeError>,
    ) -> Result<R, SegmentError> {
        let h = primary_header(self)?;
        let bytes = blob_mut(self, h.object_catalog_offset, h.object_catalog_len)?;
        let mut catalog = ObjectCatalog::read_from(bytes).map_err(map_type_err)?;
        let out = f(&mut catalog).map_err(map_type_err)?;
        catalog.write_into(bytes).map_err(map_type_err)?;
        Ok(out)
    }

    /// Inspect the live object catalog without mutating it.
    pub fn with_object_catalog<R>(
        &self,
        f: impl FnOnce(&ObjectCatalog) -> Result<R, TypeError>,
    ) -> Result<R, SegmentError> {
        let catalog = self.object_catalog()?;
        f(&catalog).map_err(map_type_err)
    }

    pub fn register_object(&mut self, row: ObjectHeader) -> Result<u32, SegmentError> {
        self.with_object_catalog_mut(|cat| cat.append_object(row))
    }
}
