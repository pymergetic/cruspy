//! Registered slab metadata.

use super::{Id, Locator};
use crate::pymergetic::cruspy::memory::segment::SegmentId;

/// Result of `Manager::register`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registered {
    pub id: Id,
    pub locator: Locator,
    pub segment_id: SegmentId,
    /// Slab index at registration time; use `Manager::slab_index` after closes.
    pub slab_index: usize,
}

/// Registered slab metadata (catalog only).
#[derive(Clone, Debug)]
pub struct MemEntry {
    pub locator: Locator,
    pub segment_id: SegmentId,
}
