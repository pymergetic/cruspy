//! Pinned catalog blobs in the primary slab arena.
//!
//! Both tables use the same wire shape ([`wire::Catalog`]), pin path ([`pin::pin_primary_catalog`]),
//! and mount sequence ([`mount_primary_catalogs`]). Content differs only by row type and FourCC.
//!
//! When a chunk fills, [`chain::append_to_chain`] allocates the next blob in talc and sets
//! `next_offset` / `next_len` on the tail header (`0` / `0` on the last chunk).
//!
//! | Table | Blob tag | Module |
//! |-------|----------|--------|
//! | Metatypes | `CTLG` | [`metatype`] |
//! | Objects | `COBJ` | [`objects`] |

mod chain;
mod metatype;
mod metatype_segment;
mod object_segment;
mod objects;
mod pin;
mod primary;
pub mod stats;
pub mod wire;

pub use metatype::{
    MetaTypeCatalog, DEFAULT_METATYPE_CATALOG_CAPACITY, METATYPE_CATALOG_HEADER_LEN,
    METATYPE_CATALOG_MAGIC, METATYPE_CATALOG_SELF_INDEX, METATYPE_CATALOG_VERSION,
};
pub use objects::{
    ObjectCatalog, DEFAULT_OBJECT_CATALOG_CAPACITY, OBJECT_CATALOG_HEADER_LEN,
    OBJECT_CATALOG_MAGIC, OBJECT_CATALOG_VERSION,
};
pub use stats::{format_memory_overview, CatalogKindStats, SegmentMemoryOverview};
pub use wire::{Catalog, CatalogKind, CatalogRow, CATALOG_HEADER_LEN};

pub(crate) use pin::{pin_in_talc, pin_primary_catalog, PinnedCatalog};

use crate::pymergetic::cruspy::io::HasSlab;
use crate::pymergetic::cruspy::memory::segment::SegmentError;
use talc::{source::Manual, TalcCell};

use metatype::MetaTypeCatalogKind;
use objects::ObjectCatalogKind;

/// Pin metatype (`CTLG`) then object (`COBJ`) catalogs on a mounted primary arena.
pub(crate) fn mount_primary_catalogs(
    talc: &mut TalcCell<Manual>,
    backend: &dyn HasSlab,
    arena_start: usize,
    arena_len: u32,
) -> Result<(u32, u32, u32, u32), SegmentError> {
    let (metatype_catalog_offset, metatype_catalog_len) =
        pin_primary_catalog::<MetaTypeCatalogKind, _>(
            talc,
            backend,
            arena_start,
            arena_len,
            0,
            |cap| MetaTypeCatalog::for_mount(cap).map_err(map_type_err),
        )?;
    let (object_catalog_offset, object_catalog_len) = pin_primary_catalog::<ObjectCatalogKind, _>(
        talc,
        backend,
        arena_start,
        arena_len,
        metatype_catalog_len as usize,
        |cap| ObjectCatalog::for_mount(cap).map_err(map_type_err),
    )?;
    Ok((
        metatype_catalog_offset,
        metatype_catalog_len,
        object_catalog_offset,
        object_catalog_len,
    ))
}

pub(crate) fn map_type_err(
    e: crate::pymergetic::cruspy::memory::types::TypeError,
) -> crate::pymergetic::cruspy::memory::segment::SegmentError {
    use crate::pymergetic::cruspy::memory::segment::SegmentError;
    use crate::pymergetic::cruspy::memory::types::TypeError;
    match e {
        TypeError::BadHeader => SegmentError::BadHeader,
        TypeError::OutOfBounds | TypeError::CapacityExceeded | TypeError::InvalidUtf8 => {
            SegmentError::CapacityRequired
        }
    }
}
