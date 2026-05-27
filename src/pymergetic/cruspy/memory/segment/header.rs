//! Fixed POD at the start of every segment.

pub const MAGIC: u32 = 0x4352_5553; // "CRUS"
pub const VERSION: u32 = 1;
pub const HEADER_LEN: usize = 512;

/// Layout prefix: arena bounds and room for fixed fields (ints, handles, …).
#[derive(Copy, Clone)]
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
