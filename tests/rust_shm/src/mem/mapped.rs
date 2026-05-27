//! Shared `mmap(MAP_SHARED)` plumbing for file + POSIX SHM.

use std::os::fd::AsFd;

use nix::sys::mman::{mmap, munmap, MapFlags, ProtFlags};

pub struct MappedRegion {
    pub ptr: *mut u8,
    pub len: usize,
}

impl MappedRegion {
    pub fn map_shared<F: AsFd>(fd: &F, len: usize) -> nix::Result<Self> {
        let mapped = unsafe {
            mmap(
                None,
                len.try_into().map_err(|_| nix::Error::EINVAL)?,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd,
                0,
            )?
        };
        Ok(Self {
            ptr: mapped.as_ptr() as *mut u8,
            len,
        })
    }
}

impl Drop for MappedRegion {
    fn drop(&mut self) {
        if self.len > 0 {
            if let Some(addr) = std::ptr::NonNull::new(self.ptr as *mut std::ffi::c_void) {
                unsafe {
                    let _ = munmap(addr, self.len);
                }
            }
            self.ptr = std::ptr::null_mut();
            self.len = 0;
        }
    }
}
