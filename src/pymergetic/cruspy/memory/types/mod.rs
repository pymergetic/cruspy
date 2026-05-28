//! Typed objects that live inside slab arenas.

mod error;
mod flex_string;
mod handle;
mod layout;
mod metatype;
mod object;
mod object_header;

pub use error::TypeError;
pub use flex_string::FlexString;
pub use handle::TypeHandle;
pub use layout::{StringHeader, STRING_HEADER_LEN, STRING_MAGIC, STRING_VERSION};
pub use metatype::{HasMetaType, MetaType, MetaTypeHeader, META_TYPE_HEADER_LEN, META_TYPE_MAGIC, META_TYPE_VERSION};
pub use object::MemoryObject;
pub use object_header::{
    ObjectHeader, OBJECT_HEADER_LEN, OBJECT_HEADER_MAGIC, OBJECT_HEADER_VERSION,
};
