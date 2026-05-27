//! Storage I/O traits: metadata, open/close, byte mapping.

pub mod access;
pub mod info;
pub mod mapping;
pub mod state;

pub use access::{HasAccess, OpenMode};
pub use info::{HasInfo, Info};
pub use mapping::HasMapping;
pub use state::{HasState, State};
