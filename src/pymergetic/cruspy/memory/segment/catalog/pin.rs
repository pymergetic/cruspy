//! Pin catalog blobs into a claimed talc arena.

use std::alloc::Layout;

use talc::{min_first_heap_layout, source::Manual, DefaultBinning, TalcCell};

use crate::pymergetic::cruspy::io::HasSlab;
use crate::pymergetic::cruspy::memory::segment::SegmentError;
use crate::pymergetic::cruspy::memory::types::TypeError;

use super::wire::{CatalogKind, CatalogRow, CATALOG_HEADER_LEN};
use super::{map_type_err, Catalog};

/// Catalog value that can be written into a pinned talc allocation.
pub(crate) trait PinnedCatalog {
    fn allocated_len(&self) -> usize;
    fn write_into(&self, dst: &mut [u8]) -> Result<(), TypeError>;
}

impl<K: CatalogKind> PinnedCatalog for Catalog<K> {
    fn allocated_len(&self) -> usize {
        Catalog::allocated_len(self)
    }

    fn write_into(&self, dst: &mut [u8]) -> Result<(), TypeError> {
        Catalog::write_into(self, dst)
    }
}

/// Row slots that fit in `arena_len` after optional bytes already reserved in the arena.
pub(crate) fn capacity_for_arena<K: CatalogKind>(
    arena_len: usize,
    row_len: usize,
    default_capacity: u32,
    already_reserved: usize,
) -> Result<u32, SegmentError> {
    use std::mem;
    use talc::base::binning::Binning;
    use talc::base::CHUNK_UNIT;
    use talc::DefaultBinning;

    let bin_count = DefaultBinning::BIN_COUNT as usize;
    let gap_lists = bin_count * mem::size_of::<Option<std::ptr::NonNull<u8>>>();
    let metadata_overhead = gap_lists
        .saturating_add(mem::size_of::<usize>())
        .saturating_add(CHUNK_UNIT * 3);
    let available = arena_len
        .saturating_sub(already_reserved)
        .saturating_sub(metadata_overhead);
    if available < CATALOG_HEADER_LEN + row_len {
        return Err(SegmentError::CapacityRequired);
    }
    let max_rows = (available - CATALOG_HEADER_LEN) / row_len;
    let cap = max_rows.min(default_capacity as usize);
    if cap == 0 {
        return Err(SegmentError::CapacityRequired);
    }
    u32::try_from(cap).map_err(|_| SegmentError::CapacityRequired)
}

/// Pin a catalog blob as the next talc allocation in the primary arena.
pub(crate) fn pin_primary_catalog<K, C>(
    talc: &mut TalcCell<Manual>,
    backend: &dyn HasSlab,
    arena_start: usize,
    arena_len: u32,
    already_reserved: usize,
    build: impl FnOnce(u32) -> Result<C, SegmentError>,
) -> Result<(u32, u32), SegmentError>
where
    K: CatalogKind,
    K::Row: CatalogRow,
    C: PinnedCatalog,
{
    let cap = capacity_for_arena::<K>(
        arena_len as usize,
        K::Row::row_len(),
        K::DEFAULT_CAPACITY,
        already_reserved,
    )?;
    let catalog = build(cap)?;
    pin_in_talc(talc, backend, &catalog, arena_start)
}

pub(crate) fn pin_in_talc(
    talc: &mut TalcCell<Manual>,
    backend: &dyn HasSlab,
    catalog: &dyn PinnedCatalog,
    arena_start: usize,
) -> Result<(u32, u32), SegmentError> {
    let reserved_len = catalog.allocated_len();
    if reserved_len < CATALOG_HEADER_LEN {
        return Err(SegmentError::CapacityRequired);
    }
    let min_layout = min_first_heap_layout::<DefaultBinning>();
    let alloc_len = reserved_len.max(min_layout.size());
    let layout =
        Layout::from_size_align(alloc_len, min_layout.align()).map_err(|_| SegmentError::CapacityRequired)?;
    let ptr = unsafe { talc.get_mut().allocate(layout) }
        .ok_or(SegmentError::CatalogAlloc)?
        .as_ptr();
    let mapping = backend.bytes();
    let ptr_usize = ptr as usize;
    let base = mapping.as_ptr() as usize;
    if ptr_usize < base.saturating_add(arena_start) || ptr_usize >= base + mapping.len() {
        return Err(SegmentError::BadHeader);
    }
    let rel = ptr_usize - base - arena_start;
    let catalog_len = u32::try_from(reserved_len).map_err(|_| SegmentError::CapacityRequired)?;
    let catalog_offset = u32::try_from(rel).map_err(|_| SegmentError::BadHeader)?;
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, alloc_len) };
    slice.fill(0);
    catalog.write_into(slice).map_err(map_type_err)?;
    Ok((catalog_offset, catalog_len))
}
