//! Central sizing defaults for segment slabs.

/// Minimum backend mapping size for any slab in a segment (enforced on open/register).
pub const MIN_SLAB_CAPACITY: usize = 512 * 1024;

/// Slab size used when `capacity` is `None` on open/create.
pub const DEFAULT_SLAB_CAPACITY: usize = MIN_SLAB_CAPACITY;
