//! Read access to an opened slab.

use super::access::Access;
use crate::mem::kind::Kind;

/// Byte view on accessed storage.
pub trait Read: Access {
    fn kind(&self) -> Kind;
    fn len(&self) -> usize;
    fn base(&self) -> *mut u8;
}
