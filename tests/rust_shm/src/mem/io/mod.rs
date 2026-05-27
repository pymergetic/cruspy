//! IO traits for memory devices.

pub mod access;
pub mod address;
pub mod file;
pub mod read;
pub mod write;

use crate::layout::Segment;

pub use access::{open_backing, Access, Open, OpenMode};
pub use address::Address;
pub use file::File;
pub use read::Read;
pub use write::Write;

/// Byte view for any opened slab (`dyn Read` or concrete device).
pub fn segment(read: &dyn Read) -> Segment<'_> {
    Segment::from_read(read)
}
