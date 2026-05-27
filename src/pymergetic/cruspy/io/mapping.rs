//! [`HasMapping`] — byte view of an opened resource.

/// Types that expose a contiguous mapped byte range while open.
pub trait HasMapping {
    fn bytes(&self) -> &[u8];
    fn bytes_mut(&mut self) -> &mut [u8];
}
