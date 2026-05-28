//! Fixed POD at the start of every slab mapping in a segment.

use std::mem;

use crate::pymergetic::cruspy::memory::wire::tags::slab;

/// Slab envelope FourCC ([`slab::CRUS`]).
pub const MAGIC: u32 = slab::CRUS;
pub const VERSION: u32 = 4;
pub const HEADER_LEN: usize = 512;

pub const SLAB_ROLE_PRIMARY: u32 = 0;
pub const SLAB_ROLE_HEAP_EXT: u32 = 1;

/// Arena claimed and (for primary) pinned metatype + object catalogs allocated in talc.
pub const FLAG_MOUNTED: u32 = 1;

/// Per-slab prefix: arena bounds + segment identity + catalog location in talc heap.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Header {
    pub magic: u32,
    pub version: u32,
    pub header_len: u32,
    pub offset: u32,
    pub len: u32,
    pub segment_uuid: [u8; 16],
    pub slab_role: u32,
    pub slab_index: u16,
    pub extension_count: u16,
    /// Arena-relative offset to pinned [`super::catalog::MetaTypeCatalog`] (`CTLG`) in talc.
    pub metatype_catalog_offset: u32,
    /// Reserved byte length of the pinned metatype catalog blob.
    pub metatype_catalog_len: u32,
    /// Arena-relative offset to pinned [`super::catalog::ObjectCatalog`] (`COBJ`) in talc.
    pub object_catalog_offset: u32,
    /// Reserved byte length of the pinned object catalog blob.
    pub object_catalog_len: u32,
    pub flags: u32,
}

impl Header {
    pub fn new_primary(
        arena_len: u32,
        segment_uuid: [u8; 16],
        metatype_catalog_offset: u32,
        metatype_catalog_len: u32,
        object_catalog_offset: u32,
        object_catalog_len: u32,
        extension_count: u16,
        flags: u32,
    ) -> Self {
        let header_len = HEADER_LEN as u32;
        Self {
            magic: MAGIC,
            version: VERSION,
            header_len,
            offset: header_len,
            len: arena_len,
            segment_uuid,
            slab_role: SLAB_ROLE_PRIMARY,
            slab_index: 0,
            extension_count,
            metatype_catalog_offset,
            metatype_catalog_len,
            object_catalog_offset,
            object_catalog_len,
            flags,
        }
    }

    pub fn new_extension(
        arena_len: u32,
        segment_uuid: [u8; 16],
        slab_index: u16,
        flags: u32,
    ) -> Self {
        let header_len = HEADER_LEN as u32;
        Self {
            magic: MAGIC,
            version: VERSION,
            header_len,
            offset: header_len,
            len: arena_len,
            segment_uuid,
            slab_role: SLAB_ROLE_HEAP_EXT,
            slab_index,
            extension_count: 0,
            metatype_catalog_offset: 0,
            metatype_catalog_len: 0,
            object_catalog_offset: 0,
            object_catalog_len: 0,
            flags,
        }
    }

    pub fn is_primary(self) -> bool {
        self.slab_role == SLAB_ROLE_PRIMARY
    }

    pub fn is_mounted(self) -> bool {
        self.flags & FLAG_MOUNTED != 0
    }
}

/// Read the POD prefix from `bytes` (unaligned-safe).
pub fn read_header(bytes: &[u8]) -> Option<Header> {
    if bytes.len() < mem::size_of::<Header>() {
        return None;
    }
    Some(unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<Header>()) })
}

/// Write `header` into the start of `bytes`.
pub fn write_header(bytes: &mut [u8], header: Header) {
    let size = mem::size_of::<Header>();
    debug_assert!(bytes.len() >= size);
    bytes[..size].copy_from_slice(unsafe {
        std::slice::from_raw_parts((&raw const header) as *const u8, size)
    });
}
