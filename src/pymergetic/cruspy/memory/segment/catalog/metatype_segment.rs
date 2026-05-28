//! [`Segment`] API for the metatype catalog (`CTLG`).

use crate::pymergetic::cruspy::memory::segment::Segment;
use crate::pymergetic::cruspy::memory::segment::SegmentError;
use crate::pymergetic::cruspy::memory::types::{HasMetaType, MetaType, MetaTypeHeader, TypeError};

use super::chain::{append_to_chain, chain_chunk_count, flatten_chain, get_in_chain, index_in_chain, load_chain};
use super::metatype::MetaTypeCatalogKind;
use super::primary::primary_header;
use super::{map_type_err, MetaTypeCatalog};

impl Segment {
    /// Flattened logical view of the full metatype chain (all chunks).
    pub fn metatype_catalog(&self) -> Result<MetaTypeCatalog, SegmentError> {
        let h = primary_header(self)?;
        let chunks = load_chain::<MetaTypeCatalogKind>(
            self,
            h.metatype_catalog_offset,
            h.metatype_catalog_len,
        )?;
        Ok(MetaTypeCatalog::from_flat(flatten_chain(&chunks)))
    }

    /// Head CTLG chunk only (preserves `next_offset` / `next_len` on the wire).
    pub fn metatype_catalog_head(&self) -> Result<MetaTypeCatalog, SegmentError> {
        let h = primary_header(self)?;
        let bytes = super::primary::blob(
            self,
            h.metatype_catalog_offset,
            h.metatype_catalog_len,
        )?;
        MetaTypeCatalog::read_from(bytes).map_err(map_type_err)
    }

    pub fn metatype_catalog_chunk_count(&self) -> Result<usize, SegmentError> {
        let h = primary_header(self)?;
        chain_chunk_count::<MetaTypeCatalogKind>(
            self,
            h.metatype_catalog_offset,
            h.metatype_catalog_len,
        )
    }

    pub fn with_metatype_catalog_mut<R>(
        &mut self,
        f: impl FnOnce(&mut MetaTypeCatalog) -> Result<R, TypeError>,
    ) -> Result<R, SegmentError> {
        let h = primary_header(self)?;
        let chunks = load_chain::<MetaTypeCatalogKind>(
            self,
            h.metatype_catalog_offset,
            h.metatype_catalog_len,
        )?;
        let mut view = MetaTypeCatalog::from_flat(flatten_chain(&chunks));
        let out = f(&mut view).map_err(map_type_err)?;
        super::chain::redistribute_chain(self, &chunks, view.inner())?;
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
        let h = primary_header(self)?;
        append_to_chain::<MetaTypeCatalogKind, _>(
            self,
            h.metatype_catalog_offset,
            h.metatype_catalog_len,
            row,
            |cap| MetaTypeCatalog::for_mount_extension(cap).map_err(map_type_err),
        )
    }

    pub fn register_metatype_for<T: HasMetaType>(&mut self) -> Result<u32, SegmentError> {
        self.register_metatype(MetaType::from_type::<T>().to_header())
    }

    pub fn ensure_metatype_registered<T: HasMetaType>(&mut self) -> Result<u32, SegmentError> {
        if let Some(index) = self.metatype_index_for::<T>()? {
            return Ok(index);
        }
        self.register_metatype_for::<T>()
    }

    pub fn metatype_index_for_uuid(
        &self,
        type_uuid: [u8; 16],
    ) -> Result<Option<u32>, SegmentError> {
        let h = primary_header(self)?;
        index_in_chain::<MetaTypeCatalogKind>(
            self,
            h.metatype_catalog_offset,
            h.metatype_catalog_len,
            |row| row.type_uuid == type_uuid,
        )
    }

    pub fn metatype_index_for<T: HasMetaType>(&self) -> Result<Option<u32>, SegmentError> {
        self.metatype_index_for_uuid(T::TYPE_UUID)
    }

    pub fn metatype_at(&self, type_index: u32) -> Result<Option<MetaTypeHeader>, SegmentError> {
        let h = primary_header(self)?;
        get_in_chain::<MetaTypeCatalogKind>(
            self,
            h.metatype_catalog_offset,
            h.metatype_catalog_len,
            type_index,
        )
    }
}
