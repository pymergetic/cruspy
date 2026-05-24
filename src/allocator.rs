use std::ffi::CStr;
use std::mem::MaybeUninit;

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::errors::{AllocationError, ShmError};
use crate::memory::MemoryHandle;
use crate::module::register_submodule;

extern "C" {
    fn cruspy_registered_type_count() -> u32;
    fn cruspy_memory_abi() -> u32;
    fn cruspy_domain_stats_count() -> u32;
    fn cruspy_domain_stats_snapshot(index: u32, out: *mut CruspyDomainStatsSnapshot) -> u32;
    fn cruspy_domain_stats_by_id(
        domain_id_high: u64,
        domain_id_low: u64,
        out: *mut CruspyDomainStatsSnapshot,
    ) -> u32;
    fn cruspy_resolve(
        handle: *const MemoryHandle,
        out_data: *mut u8,
        out_capacity: u32,
        out_size: *mut u32,
    ) -> i32;
    fn cruspy_migrate(
        handle: *const MemoryHandle,
        target_domain_high: u64,
        target_domain_low: u64,
        out: *mut MemoryHandle,
    ) -> i32;
    fn cruspy_transfer(
        handle: *const MemoryHandle,
        target_domain_high: u64,
        target_domain_low: u64,
        engine: u8,
        out: *mut MemoryHandle,
    ) -> i32;
    fn cruspy_substrate_last_error() -> *const std::ffi::c_char;
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CruspyDomainStatsSnapshot {
    name: [u8; 64],
    domain_id_high: u64,
    domain_id_low: u64,
    kind: u8,
    visibility: u8,
    residency_tier: u8,
    active: u8,
    bytes_total: u64,
    bytes_used: u64,
    object_count: u64,
    total_slots: u64,
    used_slots: u64,
    fragmentation_pct: f32,
    fullness_pct: f32,
    backing_path: [u8; 256],
    map_mode: [u8; 16],
    capabilities: u16,
}

fn cstr_from_bytes(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn substrate_error(default: &'static str) -> PyErr {
    unsafe {
        let ptr = cruspy_substrate_last_error();
        if ptr.is_null() {
            return ShmError::new_err(default);
        }
        let message = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        if message.contains("allocation") {
            AllocationError::new_err(message)
        } else {
            ShmError::new_err(message)
        }
    }
}

fn snapshot_to_domain_stats(snapshot: CruspyDomainStatsSnapshot) -> DomainStats {
    DomainStats {
        name: cstr_from_bytes(&snapshot.name),
        domain_id_high: snapshot.domain_id_high,
        domain_id_low: snapshot.domain_id_low,
        kind: snapshot.kind,
        visibility: snapshot.visibility,
        residency_tier: snapshot.residency_tier,
        active: snapshot.active != 0,
        bytes_total: snapshot.bytes_total,
        bytes_used: snapshot.bytes_used,
        object_count: snapshot.object_count,
        total_slots: snapshot.total_slots,
        used_slots: snapshot.used_slots,
        fragmentation_pct: snapshot.fragmentation_pct,
        fullness_pct: snapshot.fullness_pct,
        backing_path: cstr_from_bytes(&snapshot.backing_path),
        map_mode: cstr_from_bytes(&snapshot.map_mode),
        capabilities: snapshot.capabilities,
    }
}

#[pyclass(name = "DomainStats")]
#[derive(Clone, Debug)]
pub struct DomainStats {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub domain_id_high: u64,
    #[pyo3(get)]
    pub domain_id_low: u64,
    #[pyo3(get)]
    pub kind: u8,
    #[pyo3(get)]
    pub visibility: u8,
    #[pyo3(get)]
    pub residency_tier: u8,
    #[pyo3(get)]
    pub active: bool,
    #[pyo3(get)]
    pub bytes_total: u64,
    #[pyo3(get)]
    pub bytes_used: u64,
    #[pyo3(get)]
    pub object_count: u64,
    #[pyo3(get)]
    pub total_slots: u64,
    #[pyo3(get)]
    pub used_slots: u64,
    #[pyo3(get)]
    pub fragmentation_pct: f32,
    #[pyo3(get)]
    pub fullness_pct: f32,
    #[pyo3(get)]
    pub backing_path: String,
    #[pyo3(get)]
    pub map_mode: String,
    #[pyo3(get)]
    pub capabilities: u16,
}

#[pyclass(name = "RegistryStats")]
#[derive(Clone, Copy, Debug)]
pub struct RegistryStats {
    #[pyo3(get)]
    pub registered_count: u32,
    #[pyo3(get)]
    pub domain_count: u32,
    #[pyo3(get)]
    pub bytes_total: u64,
    #[pyo3(get)]
    pub bytes_used: u64,
    #[pyo3(get)]
    pub object_count: u64,
}

#[pyfunction]
fn stats() -> RegistryStats {
    let domain_count = unsafe { cruspy_domain_stats_count() };
    let mut bytes_total = 0;
    let mut bytes_used = 0;
    let mut object_count = 0;
    for index in 0..domain_count {
        let mut snapshot = MaybeUninit::<CruspyDomainStatsSnapshot>::uninit();
        if unsafe { cruspy_domain_stats_snapshot(index, snapshot.as_mut_ptr()) } != 0 {
            let snapshot = unsafe { snapshot.assume_init() };
            bytes_total += snapshot.bytes_total;
            bytes_used += snapshot.bytes_used;
            object_count += snapshot.object_count;
        }
    }
    RegistryStats {
        registered_count: unsafe { cruspy_registered_type_count() },
        domain_count,
        bytes_total,
        bytes_used,
        object_count,
    }
}

#[pyfunction]
fn domain_stats(domain_id_high: u64, domain_id_low: u64) -> PyResult<Option<DomainStats>> {
    let mut snapshot = MaybeUninit::<CruspyDomainStatsSnapshot>::uninit();
    let found = unsafe {
        cruspy_domain_stats_by_id(domain_id_high, domain_id_low, snapshot.as_mut_ptr())
    };
    if found == 0 {
        return Ok(None);
    }
    Ok(Some(snapshot_to_domain_stats(unsafe { snapshot.assume_init() })))
}

#[pyfunction]
fn list_domain_stats(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let count = unsafe { cruspy_domain_stats_count() };
    let list = PyList::empty(py);
    for index in 0..count {
        let mut snapshot = MaybeUninit::<CruspyDomainStatsSnapshot>::uninit();
        if unsafe { cruspy_domain_stats_snapshot(index, snapshot.as_mut_ptr()) } != 0 {
            let stats = snapshot_to_domain_stats(unsafe { snapshot.assume_init() });
            list.append(stats)?
        }
    }
    Ok(list.into())
}

#[pyfunction]
fn resolve(handle: &crate::shm::ShmHandle) -> PyResult<Vec<u8>> {
    let mut out = vec![0u8; handle.byte_size as usize];
    let mut out_size = 0u32;
    let status = unsafe {
        cruspy_resolve(
            &handle.memory,
            out.as_mut_ptr(),
            out.len() as u32,
            &mut out_size,
        )
    };
    if status != 0 {
        return Err(substrate_error("cruspy.shm: resolve failed"));
    }
    out.truncate(out_size as usize);
    Ok(out)
}

#[pyfunction]
fn migrate(handle: &crate::shm::ShmHandle, target_domain_high: u64, target_domain_low: u64) -> PyResult<crate::shm::ShmHandle> {
    let mut out = MaybeUninit::<MemoryHandle>::uninit();
    let status = unsafe {
        cruspy_migrate(
            &handle.memory,
            target_domain_high,
            target_domain_low,
            out.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(substrate_error("cruspy.shm: migrate failed"));
    }
    Ok(crate::shm::ShmHandle::from_memory(
        handle.segment.clone(),
        handle.type_fqn.clone(),
        unsafe { out.assume_init() },
    ))
}

#[pyfunction]
fn transfer(
    handle: &crate::shm::ShmHandle,
    target_domain_high: u64,
    target_domain_low: u64,
    engine: u8,
) -> PyResult<crate::shm::ShmHandle> {
    let mut out = MaybeUninit::<MemoryHandle>::uninit();
    let status = unsafe {
        cruspy_transfer(
            &handle.memory,
            target_domain_high,
            target_domain_low,
            engine,
            out.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(substrate_error("cruspy.shm: transfer failed"));
    }
    Ok(crate::shm::ShmHandle::from_memory(
        handle.segment.clone(),
        handle.type_fqn.clone(),
        unsafe { out.assume_init() },
    ))
}

pub fn memory_abi() -> u32 {
    unsafe { cruspy_memory_abi() }
}

pub fn register_allocator_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    register_submodule(
        py,
        parent,
        "pymergetic.cruspy.allocator",
        "allocator",
        |allocator| {
            allocator.add_class::<RegistryStats>()?;
            allocator.add_class::<DomainStats>()?;
            allocator.add_function(wrap_pyfunction!(stats, allocator)?)?;
            allocator.add_function(wrap_pyfunction!(domain_stats, allocator)?)?;
            allocator.add_function(wrap_pyfunction!(list_domain_stats, allocator)?)?;
            allocator.add_function(wrap_pyfunction!(resolve, allocator)?)?;
            allocator.add_function(wrap_pyfunction!(migrate, allocator)?)?;
            allocator.add_function(wrap_pyfunction!(transfer, allocator)?)?;
            Ok(())
        },
    )
}
