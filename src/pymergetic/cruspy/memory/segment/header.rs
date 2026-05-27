//! Fixed POD at the start of every segment.

use std::mem;

pub const MAGIC: u32 = 0x4352_5553; // "CRUS"
pub const VERSION: u32 = 1;
pub const HEADER_LEN: usize = 512;

/// Layout prefix: arena bounds and room for fixed fields (ints, handles, …).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Header {
    pub magic: u32,
    pub version: u32,
    /// Reserved prefix size in the mapping (today [`HEADER_LEN`]; may grow later).
    pub header_len: u32,
    /// Arena start byte offset (today equals [`header_len`](Self::header_len)).
    pub offset: u32,
    pub len: u32,
}

impl Header {
    pub fn new(arena_len: u32) -> Self {
        let header_len = HEADER_LEN as u32;
        Self {
            magic: MAGIC,
            version: VERSION,
            header_len,
            offset: header_len,
            len: arena_len,
        }
    }
}

/// Read the POD prefix from `bytes` (unaligned-safe; copies 20 bytes).
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
