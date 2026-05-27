//! [`HasResize`] — grow or shrink the raw backing mapping in place.

use super::access::HasAccess;

/// Types whose mapped byte length can change while the handle stays open.
///
/// `new_capacity` is the **total** mapping size in bytes (header prefix + arena).
/// Uses the same [`HasAccess::Error`] as open/close.
pub trait HasResize: HasAccess {
    /// Resize the raw buffer/mapping to `new_capacity` bytes (same object, new size).
    fn resize(&mut self, new_capacity: usize) -> Result<(), Self::Error>;
}
