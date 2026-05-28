use crate::pymergetic::cruspy::memory::manager::{Id, SegmentId};

/// Shared identity anchor for all custom memory-resident types.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MemoryObject {
    pub id: Id,
    pub segment_id: SegmentId,
}

impl MemoryObject {
    pub fn new(id: Id, segment_id: SegmentId) -> Self {
        Self { id, segment_id }
    }
}
