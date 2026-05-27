//! Offsets, segments, and snapshot migrate (CPU copy).
//!
//! Device IO lives in [`crate::mem::io`] ([`Read`](crate::mem::io::Read), [`Write`](crate::mem::io::Write)).

use std::fmt;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ptr;

use crate::mem::io::{Read, Write};

/// Relative address inside a mapping (portable across processes).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Off(pub u64);

impl Off {
    pub const NULL: Self = Self(u64::MAX);

    pub fn is_null(self) -> bool {
        self.0 == u64::MAX
    }
}

pub struct Segment<'a> {
    base: *mut u8,
    len: usize,
    _life: PhantomData<&'a dyn Read>,
}

impl Copy for Segment<'_> {}

impl Clone for Segment<'_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a> Segment<'a> {
    pub(crate) fn from_read(read: &'a dyn Read) -> Self {
        Self {
            base: read.base(),
            len: read.len(),
            _life: PhantomData,
        }
    }

    pub fn at<T>(&self, off: Off) -> Ref<'a, T> {
        Ref {
            seg: *self,
            off: off.0 as usize,
            _ty: PhantomData,
        }
    }

    pub(crate) fn bounds_ok<T>(&self, off: usize) -> bool {
        off.checked_add(size_of::<T>()).is_some_and(|end| end <= self.len)
    }
}

/// Typed view at `off` in the mapping.
pub struct Ref<'a, T> {
    pub(crate) seg: Segment<'a>,
    off: usize,
    _ty: PhantomData<T>,
}

impl<'a, T: Copy> Ref<'a, T> {
    pub fn off(&self) -> Off {
        Off(self.off as u64)
    }

    pub fn read(&self) -> T {
        assert!(self.seg.bounds_ok::<T>(self.off), "Ref read OOB");
        unsafe { ptr::read_unaligned(self.seg.base.add(self.off) as *const T) }
    }

    pub fn write(&self, value: T) {
        assert!(self.seg.bounds_ok::<T>(self.off), "Ref write OOB");
        unsafe { ptr::write_unaligned(self.seg.base.add(self.off) as *mut T, value) }
    }
}

pub(crate) fn align_up(x: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (x + (align - 1)) & !(align - 1)
}

// ---------------------------------------------------------------------------
// Snapshot migrate (any backend → any backend via CPU copy)
// ---------------------------------------------------------------------------

pub const SNAPSHOT_ABI: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub abi_version: u32,
    pub used_len: usize,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrateError {
    CapacityTooSmall { need: usize, have: usize },
}

impl fmt::Display for MigrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityTooSmall { need, have } => {
                write!(f, "mapping too small: need {need} bytes, have {have}")
            }
        }
    }
}

impl std::error::Error for MigrateError {}

/// Copy `used_len` bytes from any mapping into an owned buffer.
pub fn export_snapshot(mapping: &dyn Read, used_len: usize) -> Snapshot {
    let used_len = used_len.min(mapping.len());
    let mut bytes = vec![0u8; used_len];
    if used_len > 0 {
        unsafe {
            ptr::copy_nonoverlapping(mapping.base(), bytes.as_mut_ptr(), used_len);
        }
    }
    Snapshot {
        abi_version: SNAPSHOT_ABI,
        used_len,
        bytes,
    }
}

/// Copy snapshot bytes into any mapping (same layout required).
pub fn import_snapshot(mapping: &mut dyn Write, snap: &Snapshot) -> Result<(), MigrateError> {
    if snap.abi_version != SNAPSHOT_ABI {
        // demo: only one version today
    }
    if snap.used_len > mapping.len() {
        return Err(MigrateError::CapacityTooSmall {
            need: snap.used_len,
            have: mapping.len(),
        });
    }
    if snap.used_len > 0 {
        unsafe {
            ptr::copy_nonoverlapping(snap.bytes.as_ptr(), mapping.base(), snap.used_len);
        }
    }
    Ok(())
}

/// Convenience: export from `src`, import into `dst`.
pub fn migrate(
    src: &dyn Read,
    dst: &mut dyn Write,
    used_len: usize,
) -> Result<(), MigrateError> {
    let snap = export_snapshot(src, used_len);
    import_snapshot(dst, &snap)
}
