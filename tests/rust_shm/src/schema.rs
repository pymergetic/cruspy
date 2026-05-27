//! Demo nested graph — same layout on every backend.

use std::mem::{align_of, size_of};

use crate::layout::{align_up, Segment, Off, Ref};
use crate::mem::io::Write;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Deep {
    pub z: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Child {
    pub x: u32,
    pub deep: Off,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub magic: u32,
    pub child: Off,
}

impl<'a> Ref<'a, Node> {
    pub fn child(&self) -> Ref<'a, Child> {
        self.seg.at(self.read().child)
    }
}

impl<'a> Ref<'a, Child> {
    pub fn deep(&self) -> Ref<'a, Deep> {
        self.seg.at(self.read().deep)
    }
}

/// Fixed layout for Node → Child → Deep (compile-time planner; prod uses allocator).
#[derive(Copy, Clone, Debug)]
pub struct NestedLayout {
    pub node: Off,
    pub child: Off,
    pub deep: Off,
}

impl NestedLayout {
    pub fn plan() -> Self {
        let node = Off(0);
        let child = Off(align_up(size_of::<Node>(), align_of::<Child>()) as u64);
        let deep = Off(align_up(
            child.0 as usize + size_of::<Child>(),
            align_of::<Deep>(),
        ) as u64);
        Self { node, child, deep }
    }

    /// Byte length touched by this graph (for snapshot export).
    pub fn used_len(&self) -> usize {
        self.deep.0 as usize + size_of::<Deep>()
    }

    pub fn init_in(&self, mapping: &mut dyn Write, magic: u32, x: u32, z: u32) {
        let seg = crate::mem::io::segment(mapping);
        assert!(seg.bounds_ok::<Deep>(self.deep.0 as usize));

        seg.at(self.deep).write(Deep { z });
        seg.at(self.child).write(Child {
            x,
            deep: self.deep,
        });
        seg.at(self.node).write(Node {
            magic,
            child: self.child,
        });
    }

    pub fn read_root<'a>(&self, seg: Segment<'a>) -> Ref<'a, Node> {
        seg.at(self.node)
    }
}

pub fn assert_graph(node: Ref<'_, Node>, magic: u32, x: u32, z: u32) {
    assert_eq!(node.read().magic, magic);
    assert_eq!(node.child().read().x, x);
    assert_eq!(node.child().deep().read().z, z);
}
