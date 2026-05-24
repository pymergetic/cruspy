use pyo3::prelude::*;

mod allocator;
mod async_bridge;
mod core;
mod memory;
mod model_runtime;
mod module;
mod runtime;
mod schema;
mod shm;

#[path = "pymergetic/cruspy/errors/mod.gen.rs"]
pub mod errors;

#[path = "pymergetic/cruspy/models/mod.gen.rs"]
mod models;

#[link(name = "cruspy-cpp", kind = "static")]
extern "C" {
    fn cruspy_runtime_version() -> *const std::ffi::c_char;
    fn cruspy_abi_version() -> u32;
}

fn runtime_version() -> &'static str {
    unsafe {
        let ptr = cruspy_runtime_version();
        if ptr.is_null() {
            "unknown"
        } else {
            std::ffi::CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
        }
    }
}

use allocator::register_allocator_module;
use allocator::memory_abi;
use async_bridge::init_runtime;
use core::register_core_module;
use errors::register_errors_module;
use models::register_models_module;
use module::ensure_package_path;
use runtime::register_runtime_module;
use shm::register_shm_module;

#[pymodule]
#[pyo3(name = "cruspy")]
fn cruspy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_runtime();
    m.add("__doc__", "cruspy — polyglot shared-memory runtime")?;
    ensure_package_path(m)?;
    m.add("ABI_VERSION", unsafe { cruspy_abi_version() }.to_string())?;
    m.add("MEMORY_ABI", memory_abi().to_string())?;
    m.add("RUNTIME_VERSION", runtime_version())?;
    register_errors_module(m)?;
    register_core_module(m)?;
    register_allocator_module(m)?;
    register_models_module(m)?;
    register_shm_module(m)?;
    register_runtime_module(m)?;
    Ok(())
}
