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
//! [`Self::add`] is the low-level attach path: you already opened the [`HasAccess`] handle;
//! the mapping must already carry segment metadata. For a new empty mapping, use
//! [`Self::install`] (or [`Self::create`] which opens + installs).

mod header;

pub use header::{Header, HEADER_LEN, MAGIC, VERSION};

use std::mem;

use talc::{source::Manual, TalcCell};

use crate::pymergetic::cruspy::io::{HasInfo, HasMapping, HasSlab, OpenMode};
use crate::pymergetic::cruspy::utils::url::Url;

pub const DEFAULT_CAPACITY: usize = 64 * 1024;

#[derive(Debug)]
pub enum SegmentError {
    CapacityRequired,
    ArenaClaim,
    BadIndex,
    BadHeader,
}

/// [`close`] / [`unlink`] failed on a backend, or slab index out of range.
#[derive(Debug)]
pub enum SegmentTeardownError<E> {
    BadIndex,
    Backend(E),
}

/// Backend open failed, or segment layout / talc claim failed after open.
#[derive(Debug)]
pub enum SegmentOpenError<E> {
    Backend(E),
    Layout(SegmentError),
}

/// Shared allocator over one or more opened backend mappings.
pub struct Segment<B> {
    backends: Vec<B>,
    talc: TalcCell<Manual>,
}

impl<B: HasSlab> Segment<B> {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
            talc: TalcCell::new(Manual),
        }
    }

    /// Open ([`HasAccess::create`]) + [`install`] — new header on empty backing.
    pub fn create(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError<B::Error>> {
        self.open(url, OpenMode::Create, capacity)
    }

    /// Open ([`HasAccess::attach`]) + [`add`] — existing header required, never written here.
    pub fn attach(
        &mut self,
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError<B::Error>> {
        self.open(url, OpenMode::Attach, capacity)
    }

    /// Open a backend and add it to this segment; returns slab index.
    pub fn open(
        &mut self,
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<usize, SegmentOpenError<B::Error>> {
        let capacity = normalize_capacity(capacity);
        let backend = B::open(url, mode, Some(capacity))
            .map_err(SegmentOpenError::Backend)?;
        if mode == OpenMode::Attach {
            self.add(backend).map_err(SegmentOpenError::Layout)
        } else {
            self.install(backend).map_err(SegmentOpenError::Layout)
        }
    }

    /// New empty backing: write [`Header`] + claim arena. Does not open the [`Url`].
    pub fn install(&mut self, mut backend: B) -> Result<usize, SegmentError> {
        let capacity = backend.info().capacity;
        if capacity < HEADER_LEN {
            return Err(SegmentError::CapacityRequired);
        }
        if backend.bytes().len() < capacity {
            return Err(SegmentError::CapacityRequired);
        }

        let arena_len = (capacity - HEADER_LEN) as u32;
        write_header(backend.bytes_mut(), Header::new(arena_len));
        claim_arena(self, &mut backend)?;

        self.backends.push(backend);
        Ok(self.backends.len() - 1)
    }

    /// Existing backing: validate header only (no write), then claim arena.
    pub fn add(&mut self, mut backend: B) -> Result<usize, SegmentError> {
        let capacity = backend.info().capacity;
        if capacity < HEADER_LEN {
            return Err(SegmentError::CapacityRequired);
        }
        let mapping = backend.bytes();
        if mapping.len() < capacity {
            return Err(SegmentError::CapacityRequired);
        }
        check_header(mapping, capacity)?;
        claim_arena(self, &mut backend)?;

        self.backends.push(backend);
        Ok(self.backends.len() - 1)
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

    pub fn backends(&self) -> &[B] {
        &self.backends
    }

    pub fn backends_mut(&mut self) -> &mut [B] {
        &mut self.backends
    }

    pub fn backend(&self, index: usize) -> Option<&B> {
        self.backends.get(index)
    }

    pub fn backend_mut(&mut self, index: usize) -> Option<&mut B> {
        self.backends.get_mut(index)
    }

    /// Slab 0 (e.g. polyglot registry lives on this backend's header).
    pub fn primary(&self) -> Option<&B> {
        self.backend(0)
    }

    pub fn primary_mut(&mut self) -> Option<&mut B> {
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
    pub fn close(&mut self, index: usize) -> Result<(), SegmentTeardownError<B::Error>> {
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
    pub fn close_all(&mut self) -> Result<(), B::Error> {
        for backend in &mut self.backends {
            backend.close()?;
        }
        self.backends.clear();
        Ok(())
    }

    /// Close + [`HasAccess::unlink`] slab at `index`, then remove it.
    pub fn unlink(&mut self, index: usize) -> Result<(), SegmentTeardownError<B::Error>> {
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
    pub fn unlink_all(&mut self) -> Result<(), B::Error> {
        for backend in &mut self.backends {
            backend.close()?;
            backend.unlink()?;
        }
        self.backends.clear();
        Ok(())
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

fn claim_arena<B: HasInfo + HasMapping>(
    seg: &Segment<B>,
    backend: &mut B,
) -> Result<(), SegmentError> {
    let capacity = backend.info().capacity;
    let range = arena_range(backend.bytes(), capacity)?;
    let arena = &mut backend.bytes_mut()[range];
    unsafe {
        seg.talc
            .claim(arena.as_mut_ptr(), arena.len())
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
