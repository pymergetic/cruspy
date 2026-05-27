//! Segment: one or more backend slabs + shared talc over their arenas.
//!
//! **Header policy**
//!
//! - **Create** — backing is new/empty → write a fresh [`Header`], then claim the arena
//!   ([`install`]). Used by [`Self::create`] / [`Self::open`] with [`OpenMode::Create`].
//! - **Attach** — backing already has a valid header (e.g. SHM another process wrote) →
//!   **check** [`MAGIC`] / bounds, accept or [`SegmentError::BadHeader`], never overwrite.
//!   Used by [`Self::attach`] / [`Self::open`] with [`OpenMode::Attach`] and [`Self::add`].
//!
//! [`Self::add`] is the low-level attach path: you already opened the slab;
//! the mapping must already carry segment metadata. For a new empty mapping, use
//! [`Self::install`] (or [`Self::create`] which opens + installs).

mod error;
mod header;

pub use error::{SegmentError, SegmentOpenError, SegmentTeardownError};
pub use header::{Header, HEADER_LEN, MAGIC, VERSION};

use std::mem;

use talc::{source::Manual, TalcCell};

use crate::pymergetic::cruspy::io::{HasSlab, Kind, OpenMode, SlabError};
use crate::pymergetic::cruspy::utils::url::Url;

pub const DEFAULT_CAPACITY: usize = 64 * 1024;

/// Opaque id for a [`Segment`] instance (e.g. in the memory manager catalog).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SegmentId(pub u64);

/// Shared allocator over one or more opened backend mappings.
pub struct Segment {
    kind: Kind,
    backends: Vec<Box<dyn HasSlab>>,
    talc: TalcCell<Manual>,
}

impl Segment {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            backends: Vec::new(),
            talc: TalcCell::new(Manual),
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Open ([`HasSlab::open`] create path) + [`install`] — new header on empty backing.
    pub fn create(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError> {
        self.open(url, OpenMode::Create, capacity)
    }

    /// Open ([`HasSlab::open`] attach path) + [`add`] — existing header required.
    pub fn attach(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError> {
        self.open(url, OpenMode::Attach, capacity)
    }

    /// Open a backend and add it to this segment; returns slab index.
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
        if mode == OpenMode::Attach {
            self.add(backend).map_err(SegmentOpenError::Layout)
        } else {
            self.install(backend).map_err(SegmentOpenError::Layout)
        }
    }

    /// New empty backing: write [`Header`] + claim arena. HasSlab must already be open.
    pub fn install(&mut self, mut backend: Box<dyn HasSlab>) -> Result<usize, SegmentError> {
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

        let arena_len = (capacity - HEADER_LEN) as u32;
        write_header(backend.bytes_mut(), Header::new(arena_len));
        claim_arena(&self.talc, &mut *backend)?;

        self.backends.push(backend);
        Ok(self.backends.len() - 1)
    }

    /// Existing backing: validate header only (no write), then claim arena.
    pub fn add(&mut self, mut backend: Box<dyn HasSlab>) -> Result<usize, SegmentError> {
        if backend.kind() != self.kind {
            return Err(SegmentError::UnsupportedScheme(
                backend.info().url.scheme().into(),
            ));
        }
        let capacity = backend.info().capacity;
        if capacity < HEADER_LEN {
            return Err(SegmentError::CapacityRequired);
        }
        let mapping = backend.bytes();
        if mapping.len() < capacity {
            return Err(SegmentError::CapacityRequired);
        }
        check_header(mapping, capacity)?;
        claim_arena(&self.talc, &mut *backend)?;

        self.backends.push(backend);
        Ok(self.backends.len() - 1)
    }

    /// Push an already-opened slab (convenience over [`Self::add`]).
    pub fn push_slab<B: HasSlab + 'static>(&mut self, backend: B) -> Result<usize, SegmentError> {
        self.add(Box::new(backend))
    }

    /// Arena byte length (after header) for slab `index`.
    pub fn size(&self, index: usize) -> Option<usize> {
        self.arena(index).map(<[u8]>::len)
    }

    /// Full mapping byte length for slab `index`.
    pub fn size_raw(&self, index: usize) -> Option<usize> {
        self.backend(index).map(|b| b.bytes().len())
    }

    /// Sum of [`size`] over all slabs.
    pub fn size_all(&self) -> usize {
        (0..self.backends.len()).filter_map(|i| self.size(i)).sum()
    }

    /// Sum of [`size_raw`] over all slabs.
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

    /// Slab 0 (e.g. polyglot registry lives on this backend's header).
    pub fn primary(&self) -> Option<&dyn HasSlab> {
        self.backend(0)
    }

    pub fn primary_mut(&mut self) -> Option<&mut dyn HasSlab> {
        self.backend_mut(0)
    }

    pub fn header(&self, index: usize) -> Option<&Header> {
        self.backend(index).map(|b| as_header(b.bytes()))
    }

    /// Replace the header at `index` (e.g. after attach). Does not resize the slab.
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

    /// Close slab at `index` and remove it from this segment.
    pub fn close(&mut self, index: usize) -> Result<(), SegmentTeardownError> {
        let backend = self
            .backends
            .get_mut(index)
            .ok_or(SegmentTeardownError::BadIndex)?;
        backend
            .close()
            .map_err(SegmentTeardownError::Backend)?;
        self.backends.remove(index);
        Ok(())
    }

    /// Close every slab and clear the backend list (talc claims unchanged for now).
    pub fn close_all(&mut self) -> Result<(), SlabError> {
        for backend in &mut self.backends {
            backend.close()?;
        }
        self.backends.clear();
        Ok(())
    }

    /// Close + unlink slab at `index`, then remove it.
    pub fn unlink(&mut self, index: usize) -> Result<(), SegmentTeardownError> {
        let backend = self
            .backends
            .get_mut(index)
            .ok_or(SegmentTeardownError::BadIndex)?;
        backend
            .close()
            .map_err(SegmentTeardownError::Backend)?;
        backend
            .unlink()
            .map_err(SegmentTeardownError::Backend)?;
        self.backends.remove(index);
        Ok(())
    }

    /// Close + unlink every slab, then clear the backend list.
    pub fn unlink_all(&mut self) -> Result<(), SlabError> {
        for backend in &mut self.backends {
            backend.close()?;
            backend.unlink()?;
        }
        self.backends.clear();
        Ok(())
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

fn normalize_capacity(capacity: Option<usize>) -> usize {
    capacity
        .unwrap_or(DEFAULT_CAPACITY)
        .max(HEADER_LEN)
}

fn check_header(mapping: &[u8], capacity: usize) -> Result<(), SegmentError> {
    if mapping.len() < mem::size_of::<Header>() {
        return Err(SegmentError::BadHeader);
    }
    validate_header_layout(as_header(mapping), mapping.len(), capacity)?;
    Ok(())
}

/// Arena slice bounds from the on-disk [`Header`] (`offset` .. `offset + len`).
fn arena_range(mapping: &[u8], capacity: usize) -> Result<std::ops::Range<usize>, SegmentError> {
    if mapping.len() < mem::size_of::<Header>() {
        return Err(SegmentError::BadHeader);
    }
    let h = as_header(mapping);
    validate_header_layout(h, mapping.len(), capacity)?;
    let start = h.offset as usize;
    Ok(start..start + h.len as usize)
}

fn validate_header_layout(
    h: &Header,
    mapping_len: usize,
    capacity: usize,
) -> Result<(), SegmentError> {
    if h.magic != MAGIC || h.version != VERSION {
        return Err(SegmentError::BadHeader);
    }
    let header_len = h.header_len as usize;
    let offset = h.offset as usize;
    let len = h.len as usize;
    if header_len < mem::size_of::<Header>()
        || header_len > HEADER_LEN
        || offset < header_len
        || offset.saturating_add(len) > mapping_len
        || offset.saturating_add(len) > capacity
    {
        return Err(SegmentError::BadHeader);
    }
    Ok(())
}

fn claim_arena(talc: &TalcCell<Manual>, backend: &mut dyn HasSlab) -> Result<(), SegmentError> {
    let capacity = backend.info().capacity;
    let range = arena_range(backend.bytes(), capacity)?;
    let arena = &mut backend.bytes_mut()[range];
    unsafe {
        talc.claim(arena.as_mut_ptr(), arena.len())
            .ok_or(SegmentError::ArenaClaim)?;
    }
    Ok(())
}

fn write_header(segment: &mut [u8], header: Header) {
    let size = mem::size_of::<Header>();
    segment[..size].copy_from_slice(unsafe {
        std::slice::from_raw_parts((&raw const header) as *const u8, size)
    });
}

/// Borrows the fixed POD prefix at the start of the mapping (no copy).
fn as_header(segment: &[u8]) -> &Header {
    debug_assert!(segment.len() >= mem::size_of::<Header>());
    let ptr = segment.as_ptr().cast::<Header>();
    unsafe { &*ptr }
}
