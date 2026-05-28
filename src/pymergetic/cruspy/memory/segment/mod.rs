//! Segment: one or more backend slabs + shared talc over their arenas.
//!
//! **Locator model**
//!
//! - A segment is named by a **base** [`Locator`](crate::pymergetic::cruspy::memory::manager::Locator)
//!   (no `-N` suffix on the host).
//! - The **primary** slab uses that same URL; heap extensions use `host-0`, `host-1`, …
//!   (or path `stem-0.ext` for file).
//! - Type metadata: first pinned talc allocation on the primary slab ([`catalog::TypeCatalog`]);
//!   slab [`Header`] records `catalog_offset` and [`Header::FLAG_MOUNTED`].

mod catalog;
mod error;
mod header;

pub use catalog::{
    TypeCatalog, DEFAULT_TYPE_CATALOG_CAPACITY, TYPE_CATALOG_HEADER_LEN, TYPE_CATALOG_MAGIC,
    TYPE_CATALOG_SELF_INDEX, TYPE_CATALOG_VERSION,
};
pub use error::{SegmentError, SegmentOpenError, SegmentTeardownError};
pub use header::{
    read_header, write_header, Header, HEADER_LEN, FLAG_MOUNTED, MAGIC, SLAB_ROLE_HEAP_EXT,
    SLAB_ROLE_PRIMARY, VERSION,
};

use std::alloc::Layout;
use std::mem;

use talc::base::{binning::Binning, CHUNK_UNIT};
use talc::{min_first_heap_layout, source::Manual, DefaultBinning, TalcCell};

use crate::pymergetic::cruspy::io::{HasSlab, Kind, OpenMode, SlabError, State};
use crate::pymergetic::cruspy::memory::manager::Locator;
use crate::pymergetic::cruspy::memory::types::{MetaTypeHeader, TypeError, META_TYPE_HEADER_LEN};
use crate::pymergetic::cruspy::utils::url::Url;
use crate::pymergetic::cruspy::utils::uuid::Uuid;

pub const DEFAULT_CAPACITY: usize = 64 * 1024;

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
        let capacity = normalize_capacity(capacity);
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
        if capacity < HEADER_LEN {
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

        let capacity = catalog_capacity_for_arena(arena_len as usize)?;
        let catalog = TypeCatalog::for_mount(capacity).map_err(map_type_err)?;
        let (catalog_offset, catalog_len) =
            pin_catalog_in_talc(&mut self.talc, backend, &catalog, arena_start)?;

        let header = Header::new_primary(
            arena_len,
            self.segment_uuid,
            catalog_offset,
            catalog_len,
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

    /// Append a type row to the pinned catalog on the primary slab; returns `type_index`.
    pub fn register_type(&mut self, row: MetaTypeHeader) -> Result<u32, SegmentError> {
        let capacity = self.backends.first().ok_or(SegmentError::BadIndex)?.info().capacity;
        let (catalog_offset, catalog_len) = {
            let backend = self.backends.first().ok_or(SegmentError::BadIndex)?;
            let h = read_header(backend.bytes()).ok_or(SegmentError::BadHeader)?;
            if !h.is_primary() || !h.is_mounted() {
                return Err(SegmentError::BadHeader);
            }
            (h.catalog_offset, h.catalog_len)
        };
        let range = {
            let backend = self.backends.first().ok_or(SegmentError::BadIndex)?;
            arena_range(backend.bytes(), capacity)?
        };
        let off = catalog_offset as usize;
        let catalog_len = catalog_len as usize;
        if off + catalog_len > range.len() {
            return Err(SegmentError::BadHeader);
        }
        let start = range.start + off;
        let blob = &mut self.backends[0].bytes_mut()[start..start + catalog_len];
        let mut catalog = TypeCatalog::read_from(blob).map_err(map_type_err)?;
        let index = catalog.append_type(row).map_err(map_type_err)?;
        catalog.write_into(blob).map_err(map_type_err)?;
        Ok(index)
    }

    /// Read the type catalog from the primary slab (follows [`Header::catalog_offset`]).
    pub fn type_catalog(&self) -> Result<TypeCatalog, SegmentError> {
        let backend = self.backends.first().ok_or(SegmentError::BadIndex)?;
        let capacity = backend.info().capacity;
        let h = read_header(backend.bytes()).ok_or(SegmentError::BadHeader)?;
        if !h.is_primary() || !h.is_mounted() {
            return Err(SegmentError::BadHeader);
        }
        let range = arena_range(backend.bytes(), capacity)?;
        let off = h.catalog_offset as usize;
        let catalog_len = h.catalog_len as usize;
        if off + catalog_len > range.len() {
            return Err(SegmentError::BadHeader);
        }
        let start = range.start + off;
        let end = start + catalog_len;
        TypeCatalog::read_from(&backend.bytes()[start..end])
            .map_err(map_type_err)
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

/// How many type rows fit in a primary arena after talc gap-list metadata.
fn catalog_capacity_for_arena(arena_len: usize) -> Result<u32, SegmentError> {
    let bin_count = DefaultBinning::BIN_COUNT as usize;
    let gap_lists = bin_count * mem::size_of::<Option<std::ptr::NonNull<u8>>>();
    let metadata_overhead = gap_lists
        .saturating_add(mem::size_of::<usize>())
        .saturating_add(CHUNK_UNIT * 3);
    let available = arena_len.saturating_sub(metadata_overhead);
    if available < TYPE_CATALOG_HEADER_LEN + META_TYPE_HEADER_LEN {
        return Err(SegmentError::CapacityRequired);
    }
    let max_rows = (available - TYPE_CATALOG_HEADER_LEN) / META_TYPE_HEADER_LEN;
    let cap = max_rows.min(DEFAULT_TYPE_CATALOG_CAPACITY as usize);
    if cap == 0 {
        return Err(SegmentError::CapacityRequired);
    }
    u32::try_from(cap).map_err(|_| SegmentError::CapacityRequired)
}

fn map_type_err(e: TypeError) -> SegmentError {
    match e {
        TypeError::BadHeader => SegmentError::BadHeader,
        TypeError::OutOfBounds | TypeError::CapacityExceeded | TypeError::InvalidUtf8 => {
            SegmentError::CapacityRequired
        }
    }
}

fn normalize_capacity(capacity: Option<usize>) -> usize {
    capacity
        .unwrap_or(DEFAULT_CAPACITY)
        .max(HEADER_LEN)
}

fn arena_range(mapping: &[u8], capacity: usize) -> Result<std::ops::Range<usize>, SegmentError> {
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
    let catalog_len = h.catalog_len as usize;
    let catalog_off = h.catalog_offset as usize;
    if header_len < mem::size_of::<Header>()
        || header_len > HEADER_LEN
        || offset < header_len
        || offset.saturating_add(len) > mapping_len
        || offset.saturating_add(len) > capacity
    {
        return Err(SegmentError::BadHeader);
    }
    if h.is_primary() && h.is_mounted() && catalog_off.saturating_add(catalog_len) > len {
        return Err(SegmentError::BadHeader);
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

/// First talc allocation on the primary slab; never freed (pinned type catalog).
fn pin_catalog_in_talc(
    talc: &mut TalcCell<Manual>,
    backend: &dyn HasSlab,
    catalog: &TypeCatalog,
    arena_start: usize,
) -> Result<(u32, u32), SegmentError> {
    let reserved_len = catalog.allocated_len();
    if reserved_len < TYPE_CATALOG_HEADER_LEN {
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
