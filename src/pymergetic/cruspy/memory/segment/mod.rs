//! Segment: one or more backend slabs + shared talc over their arenas.
//!
//! **Locator model**
//!
//! - A segment is named by a **base** [`Locator`](crate::pymergetic::cruspy::memory::manager::Locator)
//!   (no `-N` suffix on the host).
//! - The **primary** slab uses that same URL; heap extensions use `host-0`, `host-1`, …
//!   (or path `stem-0.ext` for file).
//! - Primary mount pins both [`catalog::MetaTypeCatalog`] (`CTLG`) and [`catalog::ObjectCatalog`] (`COBJ`)
//!   in talc; slab [`Header`] records each blob’s arena offset and reserved length.

mod catalog;
mod error;
mod header;

pub use catalog::{
    format_memory_overview, Catalog, CatalogKind, CatalogRow, CatalogKindStats,
    MetaTypeCatalog, ObjectCatalog, SegmentMemoryOverview, CATALOG_HEADER_LEN,
    DEFAULT_METATYPE_CATALOG_CAPACITY, DEFAULT_OBJECT_CATALOG_CAPACITY, METATYPE_CATALOG_HEADER_LEN,
    METATYPE_CATALOG_MAGIC, METATYPE_CATALOG_SELF_INDEX, METATYPE_CATALOG_VERSION,
    OBJECT_CATALOG_HEADER_LEN, OBJECT_CATALOG_MAGIC, OBJECT_CATALOG_VERSION,
};
pub use error::{SegmentError, SegmentOpenError, SegmentTeardownError};
pub use header::{
    read_header, write_header, Header, HEADER_LEN, FLAG_MOUNTED, MAGIC, SLAB_ROLE_HEAP_EXT,
    SLAB_ROLE_PRIMARY, VERSION,
};

use std::mem;

use talc::{source::Manual, TalcCell};

use crate::pymergetic::cruspy::io::{HasSlab, Kind, OpenMode, SlabError, State};
use crate::pymergetic::cruspy::memory::manager::Locator;
use catalog::mount_primary_catalogs;
use crate::pymergetic::cruspy::utils::url::Url;
use crate::pymergetic::cruspy::utils::uuid::Uuid;

pub use crate::pymergetic::cruspy::memory::defaults::{DEFAULT_SLAB_CAPACITY, MIN_SLAB_CAPACITY};

/// Default slab mapping size when `capacity` is omitted ([`DEFAULT_SLAB_CAPACITY`]).
pub const DEFAULT_CAPACITY: usize = DEFAULT_SLAB_CAPACITY;

/// Opaque id for a [`Segment`] instance (e.g. in the memory manager catalog).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SegmentId(pub u64);

/// Shared allocator over one or more opened backend mappings.
pub struct Segment {
    kind: Kind,
    base: Option<Locator>,
    segment_uuid: [u8; 16],
    backends: Vec<Box<dyn HasSlab>>,
    talc: TalcCell<Manual>,
}

impl Segment {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            base: None,
            segment_uuid: [0u8; 16],
            backends: Vec::new(),
            talc: TalcCell::new(Manual),
        }
    }

    /// Segment bound to a base locator (primary slab URL = base).
    pub fn with_base(kind: Kind, base: Locator) -> Self {
        Self {
            kind,
            base: Some(base),
            segment_uuid: Uuid::new_v4().bytes(),
            backends: Vec::new(),
            talc: TalcCell::new(Manual),
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn base(&self) -> Option<&Locator> {
        self.base.as_ref()
    }

    pub fn segment_uuid(&self) -> [u8; 16] {
        self.segment_uuid
    }

    /// Open primary slab at [`Self::base`] (create path).
    pub fn create_primary(
        &mut self,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError> {
        let base = self
            .base
            .as_ref()
            .ok_or(SegmentOpenError::Layout(SegmentError::NoBaseLocator))?
            .clone();
        self.open(base.as_url(), OpenMode::Create, capacity)
    }

    /// Heap extension `n` (`0` → `base-0`, `1` → `base-1`, …).
    pub fn add_extension(
        &mut self,
        n: u16,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError> {
        let base = self
            .base
            .as_ref()
            .ok_or(SegmentOpenError::Layout(SegmentError::NoBaseLocator))?
            .clone();
        let url = base.extension(n);
        let idx = self.open(url.as_url(), OpenMode::Create, capacity)?;
        self.set_extension_count(self.backends.len() as u16);
        Ok(idx)
    }

    pub fn create(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError> {
        self.open(url, OpenMode::Create, capacity)
    }

    pub fn attach(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError> {
        self.open(url, OpenMode::Attach, capacity)
    }

    pub fn open(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError> {
        let capacity =
            normalize_capacity(capacity).map_err(SegmentOpenError::Layout)?;
        let mut backend = Kind::create_from_url(url)
            .map_err(|e| SegmentOpenError::UnsupportedScheme(e.0))?;
        if backend.kind() != self.kind {
            return Err(SegmentOpenError::Layout(SegmentError::UnsupportedScheme(
                url.scheme().into(),
            )));
        }
        backend
            .open(url, mode, Some(capacity))
            .map_err(SegmentOpenError::Backend)?;
        let slab_index = self.backends.len();
        self.register_slab(backend, mode == OpenMode::Create, slab_index)
            .map_err(SegmentOpenError::Layout)
    }

    pub fn install(&mut self, backend: Box<dyn HasSlab>) -> Result<usize, SegmentError> {
        let slab_index = self.backends.len();
        self.register_slab(backend, true, slab_index)
    }

    pub fn add(&mut self, backend: Box<dyn HasSlab>) -> Result<usize, SegmentError> {
        let slab_index = self.backends.len();
        self.register_slab(backend, false, slab_index)
    }

    fn register_slab(
        &mut self,
        mut backend: Box<dyn HasSlab>,
        install_header: bool,
        slab_index: usize,
    ) -> Result<usize, SegmentError> {
        if backend.kind() != self.kind {
            return Err(SegmentError::UnsupportedScheme(
                backend.info().url.scheme().into(),
            ));
        }
        let capacity = backend.info().capacity;
        if capacity < MIN_SLAB_CAPACITY.max(HEADER_LEN) {
            return Err(SegmentError::CapacityRequired);
        }
        if backend.bytes().len() < capacity {
            return Err(SegmentError::CapacityRequired);
        }

        let is_primary = slab_index == 0;
        if install_header {
            let arena_len = (capacity - HEADER_LEN) as u32;
            if is_primary {
                self.mount_primary_slab(&mut *backend, arena_len)?;
            } else {
                self.mount_extension_slab(&mut *backend, arena_len, slab_index as u16)?;
            }
        } else {
            let h = read_header(backend.bytes()).ok_or(SegmentError::BadHeader)?;
            validate_header_layout(h, backend.bytes().len(), capacity)?;
            if h.segment_uuid != self.segment_uuid {
                return Err(SegmentError::SegmentUuidMismatch);
            }
            if !h.is_mounted() {
                return Err(SegmentError::NotMounted);
            }
            claim_arena_span(&self.talc, &mut *backend, capacity)?;
        }

        self.backends.push(backend);
        Ok(slab_index)
    }

    /// Claim arena, pin type catalog as first talc allocation, write mounted primary header.
    fn mount_primary_slab(
        &mut self,
        backend: &mut dyn HasSlab,
        arena_len: u32,
    ) -> Result<(), SegmentError> {
        let capacity = backend.info().capacity;
        let arena_start = HEADER_LEN;
        claim_arena_span(&self.talc, backend, capacity)?;

        let (
            metatype_catalog_offset,
            metatype_catalog_len,
            object_catalog_offset,
            object_catalog_len,
        ) = mount_primary_catalogs(&mut self.talc, backend, arena_start, arena_len)?;

        let header = Header::new_primary(
            arena_len,
            self.segment_uuid,
            metatype_catalog_offset,
            metatype_catalog_len,
            object_catalog_offset,
            object_catalog_len,
            1,
            FLAG_MOUNTED,
        );
        write_header(backend.bytes_mut(), header);
        Ok(())
    }

    /// Claim extension arena and mark slab header mounted (heap only).
    fn mount_extension_slab(
        &mut self,
        backend: &mut dyn HasSlab,
        arena_len: u32,
        slab_index: u16,
    ) -> Result<(), SegmentError> {
        let capacity = backend.info().capacity;
        claim_arena_span(&self.talc, backend, capacity)?;
        write_header(
            backend.bytes_mut(),
            Header::new_extension(arena_len, self.segment_uuid, slab_index, FLAG_MOUNTED),
        );
        Ok(())
    }

    fn set_extension_count(&mut self, count: u16) {
        if let Some(backend) = self.backends.first_mut() {
            if let Some(mut h) = read_header(backend.bytes()) {
                h.extension_count = count;
                write_header(backend.bytes_mut(), h);
            }
        }
    }

    pub fn push_slab<B: HasSlab + 'static>(&mut self, backend: B) -> Result<usize, SegmentError> {
        self.add(Box::new(backend))
    }

    pub fn size(&self, index: usize) -> Option<usize> {
        self.arena(index).map(<[u8]>::len)
    }

    pub fn size_raw(&self, index: usize) -> Option<usize> {
        self.backend(index).map(|b| b.bytes().len())
    }

    pub fn size_all(&self) -> usize {
        (0..self.backends.len()).filter_map(|i| self.size(i)).sum()
    }

    pub fn size_raw_all(&self) -> usize {
        (0..self.backends.len()).filter_map(|i| self.size_raw(i)).sum()
    }

    pub fn backends(&self) -> &[Box<dyn HasSlab>] {
        &self.backends
    }

    pub fn backends_mut(&mut self) -> &mut [Box<dyn HasSlab>] {
        &mut self.backends
    }

    pub fn backend(&self, index: usize) -> Option<&dyn HasSlab> {
        self.backends.get(index).map(|b| &**b)
    }

    pub fn backend_mut(&mut self, index: usize) -> Option<&mut dyn HasSlab> {
        self.backends.get_mut(index).map(|b| &mut **b)
    }

    pub fn primary(&self) -> Option<&dyn HasSlab> {
        self.backend(0)
    }

    pub fn primary_mut(&mut self) -> Option<&mut dyn HasSlab> {
        self.backend_mut(0)
    }

    pub fn header(&self, index: usize) -> Option<Header> {
        self.backend(index)
            .and_then(|b| read_header(b.bytes()))
    }

    pub fn set_header(&mut self, index: usize, header: Header) -> bool {
        self.backend_mut(index)
            .map(|b| write_header(b.bytes_mut(), header))
            .is_some()
    }

    pub fn arena(&self, index: usize) -> Option<&[u8]> {
        self.backend(index).and_then(|b| {
            arena_range(b.bytes(), b.info().capacity)
                .ok()
                .map(|r| &b.bytes()[r])
        })
    }

    pub fn arena_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        let b = self.backend_mut(index)?;
        let capacity = b.info().capacity;
        let range = arena_range(b.bytes(), capacity).ok()?;
        Some(&mut b.bytes_mut()[range])
    }

    pub fn talc(&self) -> &TalcCell<Manual> {
        &self.talc
    }

    pub fn talc_mut(&mut self) -> &mut TalcCell<Manual> {
        &mut self.talc
    }

    /// Pin a catalog blob into the primary slab arena (talc + backend field split borrow).
    pub(crate) fn pin_catalog_on_primary(
        &mut self,
        catalog: &dyn catalog::PinnedCatalog,
    ) -> Result<(u32, u32), SegmentError> {
        let arena_start = {
            let backend = self.backends().first().ok_or(SegmentError::BadIndex)?;
            arena_range(backend.bytes(), backend.info().capacity)?.start
        };
        if self.backends.is_empty() {
            return Err(SegmentError::BadIndex);
        }
        let backend_ptr = self.backends.as_mut_ptr();
        let talc = self.talc_mut();
        // SAFETY: `pin_in_talc` allocates in talc only; it does not reallocate `backends`.
        let backend = unsafe { &*backend_ptr };
        catalog::pin_in_talc(talc, backend.as_ref(), catalog, arena_start)
    }

    pub fn close(&mut self, index: usize) -> Result<(), SegmentTeardownError> {
        let backend = self
            .backends
            .get_mut(index)
            .ok_or(SegmentTeardownError::BadIndex)?;
        backend.info_mut().state = State::Closed;
        Ok(())
    }

    pub fn close_all(&mut self) -> Result<(), SlabError> {
        for backend in &mut self.backends {
            backend.info_mut().state = State::Closed;
        }
        Ok(())
    }

    pub fn unlink(&mut self, index: usize) -> Result<(), SegmentTeardownError> {
        self.close(index)
    }

    pub fn unlink_all(&mut self) -> Result<(), SlabError> {
        self.close_all()
    }

    pub fn locate_slab(&self, locator: &Url) -> Option<usize> {
        self.backends
            .iter()
            .position(|b| b.info().url == *locator)
    }

    pub fn slab_count(&self) -> usize {
        self.backends.len()
    }

}

impl Drop for Segment {
    fn drop(&mut self) {
        for backend in &mut self.backends {
            let _ = backend.close();
            let _ = backend.unlink();
        }
    }
}

fn normalize_capacity(capacity: Option<usize>) -> Result<usize, SegmentError> {
    let cap = capacity.unwrap_or(DEFAULT_CAPACITY);
    if cap < MIN_SLAB_CAPACITY {
        return Err(SegmentError::CapacityRequired);
    }
    Ok(cap.max(HEADER_LEN))
}

pub(crate) fn arena_range(
    mapping: &[u8],
    capacity: usize,
) -> Result<std::ops::Range<usize>, SegmentError> {
    let h = read_header(mapping).ok_or(SegmentError::BadHeader)?;
    validate_header_layout(h, mapping.len(), capacity)?;
    let start = h.offset as usize;
    Ok(start..start + h.len as usize)
}

fn validate_header_layout(
    h: Header,
    mapping_len: usize,
    capacity: usize,
) -> Result<(), SegmentError> {
    if h.magic != MAGIC || h.version != VERSION {
        return Err(SegmentError::BadHeader);
    }
    let header_len = h.header_len as usize;
    let offset = h.offset as usize;
    let len = h.len as usize;
    let metatype_catalog_len = h.metatype_catalog_len as usize;
    let metatype_catalog_off = h.metatype_catalog_offset as usize;
    if header_len < mem::size_of::<Header>()
        || header_len > HEADER_LEN
        || offset < header_len
        || offset.saturating_add(len) > mapping_len
        || offset.saturating_add(len) > capacity
    {
        return Err(SegmentError::BadHeader);
    }
    if h.is_primary() && h.is_mounted() {
        let object_off = h.object_catalog_offset as usize;
        let object_len = h.object_catalog_len as usize;
        if metatype_catalog_len == 0
            || object_len == 0
            || metatype_catalog_off.saturating_add(metatype_catalog_len) > len
            || object_off.saturating_add(object_len) > len
        {
            return Err(SegmentError::BadHeader);
        }
    }
    Ok(())
}

fn claim_arena_span(
    talc: &TalcCell<Manual>,
    backend: &mut dyn HasSlab,
    capacity: usize,
) -> Result<(), SegmentError> {
    claim_subrange(talc, backend, HEADER_LEN..capacity)
}

fn claim_subrange(
    talc: &TalcCell<Manual>,
    backend: &mut dyn HasSlab,
    range: std::ops::Range<usize>,
) -> Result<(), SegmentError> {
    let len = range.end.saturating_sub(range.start);
    if len == 0 {
        return Ok(());
    }
    let ptr = backend.bytes_mut()[range].as_mut_ptr();
    unsafe {
        talc.claim(ptr, len)
            .ok_or(SegmentError::ArenaClaim)?;
    }
    backend.set_arena_claimed(true);
    Ok(())
}
