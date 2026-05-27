//! [`HasSlab`] — marker for a full segment backend (metadata + open + map + resize).

use super::{HasAccess, HasInfo, HasMapping, HasResize};

/// Segment slab: implements the full [`crate::pymergetic::cruspy::io`] stack.
///
/// Used as `Segment<B>`'s bound instead of listing four traits. Not object-safe
/// (`HasAccess::open` returns `Self`); mixed ram/shm/file segments need a separate
/// object-safe trait later.
pub trait HasSlab: HasInfo + HasAccess + HasMapping + HasResize {}

impl<T> HasSlab for T where T: HasInfo + HasAccess + HasMapping + HasResize {}
