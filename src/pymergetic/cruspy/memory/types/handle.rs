use super::MemoryObject;

/// Stable runtime handle to an object payload in a slab arena.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeHandle {
    pub object: MemoryObject,
    pub slab_index: usize,
    pub offset: usize,
    pub len: usize,
}

impl TypeHandle {
    pub fn new(
        object: MemoryObject,
        slab_index: usize,
        offset: usize,
        len: usize,
    ) -> Self {
        Self {
            object,
            slab_index,
            offset,
            len,
        }
    }

    pub fn end(self) -> usize {
        self.offset.saturating_add(self.len)
    }
}
