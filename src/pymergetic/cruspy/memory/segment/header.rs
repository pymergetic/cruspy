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
    pub offset: u32,
    pub len: u32,
}

impl Header {
    pub fn new(offset: u32, len: u32) -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            offset,
            len,
        }
    }
}
