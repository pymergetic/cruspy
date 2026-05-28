//! [`Segment`] API for the metatype catalog (`CTLG`).

use crate::pymergetic::cruspy::memory::segment::Segment;
use crate::pymergetic::cruspy::memory::segment::SegmentError;
use crate::pymergetic::cruspy::memory::types::{HasMetaType, MetaTypeHeader, TypeError};

use super::primary::{blob, blob_mut, primary_header};
use super::{map_type_err, MetaTypeCatalog};

impl Segment {
    pub fn metatype_catalog(&self) -> Result<MetaTypeCatalog, SegmentError> {
        let h = primary_header(self)?;
        let bytes = blob(self, h.metatype_catalog_offset, h.metatype_catalog_len)?;
        MetaTypeCatalog::read_from(bytes).map_err(map_type_err)
    }

    pub fn with_metatype_catalog_mut<R>(
        &mut self,
        f: impl FnOnce(&mut MetaTypeCatalog) -> Result<R, TypeError>,
    ) -> Result<R, SegmentError> {
        let h = primary_header(self)?;
        let bytes = blob_mut(self, h.metatype_catalog_offset, h.metatype_catalog_len)?;
        let mut catalog = MetaTypeCatalog::read_from(bytes).map_err(map_type_err)?;
        let out = f(&mut catalog).map_err(map_type_err)?;
        catalog.write_into(bytes).map_err(map_type_err)?;
        Ok(out)
    }

    pub fn with_metatype_catalog<R>(
        &self,
        f: impl FnOnce(&MetaTypeCatalog) -> Result<R, TypeError>,
    ) -> Result<R, SegmentError> {
        let catalog = self.metatype_catalog()?;
        f(&catalog).map_err(map_type_err)
    }

    pub fn register_metatype(&mut self, row: MetaTypeHeader) -> Result<u32, SegmentError> {
        self.with_metatype_catalog_mut(|cat| cat.append(row))
    }

    pub fn register_metatype_for<T: HasMetaType>(&mut self) -> Result<u32, SegmentError> {
        self.with_metatype_catalog_mut(|cat| cat.register_for::<T>())
    }

    pub fn ensure_metatype_registered<T: HasMetaType>(&mut self) -> Result<u32, SegmentError> {
        self.with_metatype_catalog_mut(|cat| cat.ensure_registered::<T>())
    }

    pub fn metatype_index_for_uuid(
        &self,
        type_uuid: [u8; 16],
    ) -> Result<Option<u32>, SegmentError> {
        self.with_metatype_catalog(|cat| Ok(cat.index_for_uuid(type_uuid)))
    }

    pub fn metatype_index_for<T: HasMetaType>(&self) -> Result<Option<u32>, SegmentError> {
        self.with_metatype_catalog(|cat| Ok(cat.index_for::<T>()))
    }
}
