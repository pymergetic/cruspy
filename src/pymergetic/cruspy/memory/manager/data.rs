//! Registered slab metadata.

use super::Locator;
use crate::pymergetic::cruspy::memory::segment::SegmentId;

/// Registered slab metadata (catalog only).
#[derive(Clone, Debug)]
pub struct MemEntry {
    pub locator: Locator,
    pub segment_id: SegmentId,
    pub used_len: usize,
}
