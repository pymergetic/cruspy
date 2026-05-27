//! Segment: one or more backend slabs + shared talc over their arenas.

mod header;

pub use header::{Header, HEADER_LEN, MAGIC, VERSION};

use std::mem;

use talc::{source::Manual, TalcCell};

use crate::pymergetic::cruspy::memory::backend::Backend;
use crate::pymergetic::cruspy::io::OpenMode;
use crate::pymergetic::cruspy::utils::url::Url;

pub const DEFAULT_CAPACITY: usize = 64 * 1024;

#[derive(Debug)]
pub enum SegmentError {
    CapacityRequired,
    ArenaClaim,
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

impl<B: Backend> Segment<B> {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
            talc: TalcCell::new(Manual),
        }
    }

    /// Open backend ([`HasAccess::create`]) and [`add`] it.
    pub fn create(
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<Self, SegmentOpenError<B::Error>> {
        Self::open(url, OpenMode::Create, capacity)
    }

    /// Open backend ([`HasAccess::attach`]) and [`add`] it.
    ///
    /// Today this rewrites the slab header like create; attach-only reclaim comes later.
    pub fn attach(
        url: &Url,
        capacity: Option<usize>,
    ) -> Result<Self, SegmentOpenError<B::Error>> {
        Self::open(url, OpenMode::Attach, capacity)
    }

    pub fn open(
        url: &Url,
        mode: OpenMode,
        capacity: Option<usize>,
    ) -> Result<Self, SegmentOpenError<B::Error>> {
        let capacity = capacity
            .unwrap_or(DEFAULT_CAPACITY)
            .max(HEADER_LEN);
        let backend = B::open(url, mode, Some(capacity))
            .map_err(SegmentOpenError::Backend)?;
        let mut seg = Self::new();
        seg.add(backend).map_err(SegmentOpenError::Layout)?;
        Ok(seg)
    }

    /// Segment containing a single slab (same as `new()` + [`add`]).
    pub fn with_backend(backend: B) -> Result<Self, SegmentError> {
        let mut seg = Self::new();
        seg.add(backend)?;
        Ok(seg)
    }

    /// Lay out this slab's header and [`claim`](TalcCell::claim) its arena into the shared talc.
    pub fn add(&mut self, mut backend: B) -> Result<(), SegmentError> {
        let capacity = backend.info().capacity;
        if capacity < HEADER_LEN {
            return Err(SegmentError::CapacityRequired);
        }

        let offset = HEADER_LEN as u32;
        let len = (capacity - HEADER_LEN) as u32;
        write_header(backend.bytes_mut(), Header::new(offset, len));

        let arena = &mut backend.bytes_mut()[HEADER_LEN..];
        unsafe {
            self.talc
                .claim(arena.as_mut_ptr(), arena.len())
                .ok_or(SegmentError::ArenaClaim)?;
        }

        self.backends.push(backend);
        Ok(())
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
        self.backend(index)
            .map(|b| &b.bytes()[HEADER_LEN..])
    }

    pub fn arena_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        self.backend_mut(index)
            .map(|b| &mut b.bytes_mut()[HEADER_LEN..])
    }

    pub fn talc(&self) -> &TalcCell<Manual> {
        &self.talc
    }

    pub fn close(&mut self) -> Result<(), B::Error> {
        for backend in &mut self.backends {
            backend.close()?;
        }
        self.backends.clear();
        Ok(())
    }

    pub fn unlink(&mut self) -> Result<(), B::Error> {
        for backend in &mut self.backends {
            backend.unlink()?;
        }
        Ok(())
    }
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
