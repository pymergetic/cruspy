//! Storage I/O traits: metadata, open/close, byte mapping, resize.

pub mod access;
pub mod info;
pub mod mapping;
pub mod resize;
pub mod slab;
pub mod state;

pub use access::{HasAccess, OpenMode};
pub use info::{HasInfo, Info};
pub use mapping::HasMapping;
pub use resize::HasResize;
pub use slab::HasSlab;
pub use state::{HasState, State};
