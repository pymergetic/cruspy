//! pymergetic.cruspy.shm — Rust-owned POSIX SHM domain (EP-0019)

use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};

use nix::fcntl::OFlag;
use nix::sys::mman::{mmap, shm_open, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use nix::unistd::ftruncate;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::cruspy_root::runtime::kernel::{DomainId, MemoryHandle};

const SHM_CAPACITY: usize = 16 * 1024 * 1024;
const DOMAIN_HIGH: u64 = 0x637275737079; // "cruspy"

#[repr(C)]
pub struct ShmDomainOps {
    pub ctx: *mut std::ffi::c_void,
    pub allocate: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u64, *mut MemoryHandle) -> i32>,
    pub deallocate: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const MemoryHandle) -> i32>,
    pub resolve: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const MemoryHandle) -> *mut u8>,
}

struct Slot {
    offset: usize,
    size: usize,
    live: bool,
    generation: u64,
}

struct ShmState {
    ptr: *mut u8,
    capacity: usize,
    owner: bool,
    domain_id_low: u64,
    slots: Vec<Slot>,
    bump: usize,
    shm_name: CString,
}

unsafe impl Send for ShmState {}
unsafe impl Sync for ShmState {}

static SHM: OnceLock<Mutex<ShmState>> = OnceLock::new();

extern "C" {
    fn cruspy_allocator_register_shm(
        name: *const c_char,
        ops: ShmDomainOps,
        domain_low_out: *mut u64,
    ) -> i32;
}

fn posix_name(domain: &str) -> CString {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    domain.hash(&mut hasher);
    CString::new(format!("/cruspy_{:x}", hasher.finish())).expect("shm name")
}

fn create_mapping(name: &CString) -> Result<(*mut u8, usize, bool), i32> {
    let mode = Mode::S_IRUSR | Mode::S_IWUSR;
    let fd = match shm_open(
        name.as_c_str(),
        OFlag::O_CREAT | OFlag::O_RDWR,
        mode,
    ) {
        Ok(fd) => fd,
        Err(_) => return Err(-2),
    };
    if ftruncate(&fd, SHM_CAPACITY as i64).is_err() {
        return Err(-3);
    }
    let mapped = unsafe {
        mmap(
            None,
            SHM_CAPACITY.try_into().map_err(|_| -3)?,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
            fd,
            0,
        )
    }
    .map_err(|_| -3)?;
    let ptr = mapped.as_ptr() as *mut u8;
    Ok((ptr, SHM_CAPACITY, true))
}

fn attach_mapping(name: &CString) -> Result<(*mut u8, usize, bool), i32> {
    let fd = shm_open(name.as_c_str(), OFlag::O_RDWR, Mode::empty()).map_err(|_| -2)?;
    let mapped = unsafe {
        mmap(
            None,
            SHM_CAPACITY.try_into().map_err(|_| -3)?,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_SHARED,
            fd,
            0,
        )
    }
    .map_err(|_| -3)?;
    let ptr = mapped.as_ptr() as *mut u8;
    Ok((ptr, SHM_CAPACITY, false))
}

fn with_state<F, R>(f: F) -> Result<R, i32>
where
    F: FnOnce(&mut ShmState) -> Result<R, i32>,
{
    let mut guard = SHM.get().ok_or(-1)?.lock().map_err(|_| -1)?;
    f(&mut guard)
}

unsafe extern "C" fn shm_allocate(_ctx: *mut std::ffi::c_void, size: u64, out: *mut MemoryHandle) -> i32 {
    if out.is_null() || size == 0 {
        return -1;
    }
    with_state(|state| {
        let size = size as usize;
        if state.bump + size > state.capacity {
            return Err(-4);
        }
        let offset = state.slots.len();
        let slot_offset = state.bump;
        state.slots.push(Slot {
            offset: slot_offset,
            size,
            live: true,
            generation: 0,
        });
        state.bump += size;

        (*out).abi_version = 1;
        (*out).flags = 0x03;
        (*out).domain_id = DomainId {
            high: DOMAIN_HIGH,
            low: state.domain_id_low,
        };
        (*out).offset = offset as u64;
        (*out).byte_size = size as u64;
        (*out).schema_hash = 0;
        (*out).generation = 0;
        (*out).embedded_offset = 0;
        (*out).type_fqn = [0; 24];
        Ok(0)
    })
    .unwrap_or(-1)
}

unsafe extern "C" fn shm_deallocate(_ctx: *mut std::ffi::c_void, handle: *const MemoryHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }
    with_state(|state| {
        let handle = &*handle;
        if handle.domain_id.low != state.domain_id_low {
            return Err(-2);
        }
        let index = handle.offset as usize;
        if index >= state.slots.len() {
            return Err(-2);
        }
        let slot = &mut state.slots[index];
        if !slot.live || slot.generation != handle.generation {
            return Err(-3);
        }
        slot.live = false;
        slot.generation += 1;
        Ok(0)
    })
    .unwrap_or(-1)
}

unsafe extern "C" fn shm_resolve(_ctx: *mut std::ffi::c_void, handle: *const MemoryHandle) -> *mut u8 {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(mut guard) = SHM.get().ok_or(()).and_then(|m| m.lock().map_err(|_| ())) else {
        return std::ptr::null_mut();
    };
    let state = &mut *guard;
    let handle = &*handle;
    if handle.domain_id.low != state.domain_id_low {
        return std::ptr::null_mut();
    }
    let index = handle.offset as usize;
    if index >= state.slots.len() {
        return std::ptr::null_mut();
    }
    let slot = &state.slots[index];
    if !slot.live || slot.generation != handle.generation {
        return std::ptr::null_mut();
    }
    state.ptr.add(slot.offset)
}

fn init_shm_domain() {
    if SHM.get().is_some() {
        return;
    }
    let shm_name = posix_name("shm_default");
    let (ptr, capacity, owner) = create_mapping(&shm_name).unwrap_or((std::ptr::null_mut(), 0, false));
    let mut domain_id_low = 0u64;
    let ops = ShmDomainOps {
        ctx: std::ptr::null_mut(),
        allocate: Some(shm_allocate),
        deallocate: Some(shm_deallocate),
        resolve: Some(shm_resolve),
    };
    let name = CString::new("shm_default").expect("domain name");
    let rc = unsafe {
        cruspy_allocator_register_shm(name.as_ptr(), ops, &mut domain_id_low as *mut u64)
    };
    assert!(rc == 0, "failed to register shm_default domain");

    let _ = SHM.set(Mutex::new(ShmState {
        ptr,
        capacity,
        owner,
        domain_id_low,
        slots: Vec::new(),
        bump: 0,
        shm_name,
    }));
}

pub fn attach_shm_domain(name: &str) -> Result<(), i32> {
    if SHM.get().is_some() {
        return Ok(());
    }
    let shm_name = posix_name(name);
    let (ptr, capacity, owner) = attach_mapping(&shm_name)?;
    let mut domain_id_low = 0u64;
    let ops = ShmDomainOps {
        ctx: std::ptr::null_mut(),
        allocate: Some(shm_allocate),
        deallocate: Some(shm_deallocate),
        resolve: Some(shm_resolve),
    };
    let cname = CString::new(name).map_err(|_| -1)?;
    let rc = unsafe {
        cruspy_allocator_register_shm(cname.as_ptr(), ops, &mut domain_id_low as *mut u64)
    };
    if rc != 0 {
        return Err(rc);
    }
    let _ = SHM.set(Mutex::new(ShmState {
        ptr,
        capacity,
        owner,
        domain_id_low,
        slots: Vec::new(),
        bump: 0,
        shm_name,
    }));
    Ok(())
}

// Process-lifetime mapping; unmapped when the interpreter exits.

pub fn register(_m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_shm_domain();
    Ok(())
}
